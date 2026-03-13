import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { TunnelInfo, TunnelStatusEvent, TunnelConfig, TunnelInput } from "../types";

export function useTunnels() {
  const [tunnels, setTunnels] = useState<TunnelInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [passphrasePrompt, setPassphrasePrompt] = useState<{
    tunnelId: string;
    tunnelName: string;
  } | null>(null);
  const [hostKeyPrompt, setHostKeyPrompt] = useState<{
    tunnelId: string;
    host: string;
    port: number;
    keyType: string;
    fingerprint: string;
    isChanged: boolean;
  } | null>(null);
  const [passwordPrompt, setPasswordPrompt] = useState<{
    tunnelId: string;
    tunnelName: string;
  } | null>(null);
  const [kiPrompt, setKiPrompt] = useState<{
    tunnelId: string;
    name: string;
    instructions: string;
    prompts: Array<{ text: string; echo: boolean }>;
  } | null>(null);

  const fetchTunnels = useCallback(async () => {
    try {
      const result = await invoke<TunnelInfo[]>("list_tunnels");
      setTunnels(result);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchTunnels();

    const unlisten = listen<TunnelStatusEvent>(
      "tunnel-status-changed",
      () => {
        fetchTunnels();
      }
    );

    const unlistenKi = listen<{
      tunnelId: string;
      name: string;
      instructions: string;
      prompts: Array<{ text: string; echo: boolean }>;
    }>("keyboard-interactive-prompt", (event) => {
      setKiPrompt(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenKi.then((fn) => fn());
    };
  }, [fetchTunnels]);

  const connect = useCallback(async (id: string) => {
    try {
      setError(null);
      await invoke("connect_tunnel", { id });
    } catch (e) {
      const errMsg = String(e);
      if (errMsg.includes("encrypted") || errMsg.includes("passphrase")) {
        const name = tunnels.find((t) => t.id === id)?.name ?? id;
        setPassphrasePrompt({ tunnelId: id, tunnelName: name });
      } else if (errMsg.startsWith("UNKNOWN_HOST_KEY:")) {
        // Format: UNKNOWN_HOST_KEY:host:port:key_type:fingerprint
        const parts = errMsg.split(":");
        setHostKeyPrompt({
          tunnelId: id,
          host: parts[1],
          port: parseInt(parts[2]),
          keyType: parts[3],
          fingerprint: parts.slice(4).join(":"), // fingerprint may contain colons
          isChanged: false,
        });
      } else if (errMsg.startsWith("HOST_KEY_CHANGED:")) {
        // Extract host:port from the message
        const match = errMsg.match(/host key for ([^:]+):(\d+)/i);
        setHostKeyPrompt({
          tunnelId: id,
          host: match?.[1] ?? "unknown",
          port: parseInt(match?.[2] ?? "22"),
          keyType: "",
          fingerprint: "",
          isChanged: true,
        });
      } else if (errMsg.startsWith("PASSWORD_REQUIRED:")) {
        const tunnelId = errMsg.substring("PASSWORD_REQUIRED:".length);
        const name = tunnels.find((t) => t.id === tunnelId)?.name ?? tunnelId;
        setPasswordPrompt({ tunnelId, tunnelName: name });
      } else {
        setError(errMsg);
      }
    }
  }, [tunnels]);

  const submitPassphrase = useCallback(
    async (passphrase: string) => {
      if (!passphrasePrompt) return;
      const { tunnelId } = passphrasePrompt;
      setPassphrasePrompt(null);
      try {
        await invoke("store_passphrase_for_tunnel", {
          id: tunnelId,
          passphrase,
        });
        // Retry connect now that passphrase is in keychain
        await invoke("connect_tunnel", { id: tunnelId });
      } catch (e) {
        setError(String(e));
      }
    },
    [passphrasePrompt]
  );

  const cancelPassphrase = useCallback(() => {
    setPassphrasePrompt(null);
  }, []);

  const submitPassword = useCallback(
    async (password: string) => {
      if (!passwordPrompt) return;
      const { tunnelId } = passwordPrompt;
      setPasswordPrompt(null);
      try {
        await invoke("store_password_for_tunnel", { id: tunnelId, password });
        await invoke("connect_tunnel", { id: tunnelId });
      } catch (e) {
        setError(String(e));
      }
    },
    [passwordPrompt]
  );

  const cancelPassword = useCallback(() => {
    setPasswordPrompt(null);
  }, []);

  const acceptHostKey = useCallback(async () => {
    if (!hostKeyPrompt) return;
    const { tunnelId, host, port } = hostKeyPrompt;
    setHostKeyPrompt(null);
    try {
      await invoke("accept_host_key", { host, port });
      // Retry connect now that the key is in known_hosts
      await invoke("connect_tunnel", { id: tunnelId });
    } catch (e) {
      setError(String(e));
    }
  }, [hostKeyPrompt]);

  const rejectHostKey = useCallback(() => {
    setHostKeyPrompt(null);
  }, []);

  const respondKeyboardInteractive = useCallback(
    async (responses: string[]) => {
      if (!kiPrompt) return;
      const { tunnelId } = kiPrompt;
      setKiPrompt(null);
      try {
        await invoke("respond_keyboard_interactive", { id: tunnelId, responses });
      } catch (e) {
        setError(String(e));
      }
    },
    [kiPrompt]
  );

  const cancelKeyboardInteractive = useCallback(async () => {
    if (!kiPrompt) return;
    const { tunnelId } = kiPrompt;
    setKiPrompt(null);
    try {
      await invoke("cancel_keyboard_interactive", { id: tunnelId });
    } catch (e) {
      setError(String(e));
    }
  }, [kiPrompt]);

  const disconnect = useCallback(async (id: string) => {
    try {
      await invoke("disconnect_tunnel", { id });
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const reload = useCallback(async () => {
    try {
      await invoke("reload_config");
      await fetchTunnels();
    } catch (e) {
      setError(String(e));
    }
  }, [fetchTunnels]);

  const addTunnel = useCallback(async (input: TunnelInput) => {
    try {
      setError(null);
      await invoke("add_tunnel", { input });
      await fetchTunnels();
    } catch (e) {
      setError(String(e));
      throw e;
    }
  }, [fetchTunnels]);

  const updateTunnel = useCallback(async (id: string, input: TunnelInput) => {
    try {
      setError(null);
      await invoke("update_tunnel", { id, input });
      await fetchTunnels();
    } catch (e) {
      setError(String(e));
      throw e;
    }
  }, [fetchTunnels]);

  const deleteTunnel = useCallback(async (id: string) => {
    try {
      setError(null);
      await invoke("delete_tunnel", { id });
      await fetchTunnels();
    } catch (e) {
      setError(String(e));
      throw e;
    }
  }, [fetchTunnels]);

  const getTunnelConfig = useCallback(async (id: string): Promise<TunnelConfig> => {
    return await invoke<TunnelConfig>("get_tunnel_config", { id });
  }, []);

  return {
    tunnels,
    loading,
    error,
    connect,
    disconnect,
    reload,
    passphrasePrompt,
    submitPassphrase,
    cancelPassphrase,
    hostKeyPrompt,
    acceptHostKey,
    rejectHostKey,
    passwordPrompt,
    submitPassword,
    cancelPassword,
    kiPrompt,
    respondKeyboardInteractive,
    cancelKeyboardInteractive,
    addTunnel,
    updateTunnel,
    deleteTunnel,
    getTunnelConfig,
  };
}