import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { TunnelInfo, TunnelStatusEvent } from "../types";

export function useTunnels() {
  const [tunnels, setTunnels] = useState<TunnelInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [passphrasePrompt, setPassphrasePrompt] = useState<{
    tunnelId: string;
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
      (_event) => {
        fetchTunnels();
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [fetchTunnels]);

  const connect = useCallback(async (id: string) => {
    try {
      setError(null);
      await invoke("connect_tunnel", { id });
    } catch (e) {
      const errMsg = String(e);
      if (errMsg.includes("encrypted") || errMsg.includes("passphrase")) {
        setPassphrasePrompt({ tunnelId: id });
      } else {
        setError(errMsg);
      }
    }
  }, []);

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
  };
}
