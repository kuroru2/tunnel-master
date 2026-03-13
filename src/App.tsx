import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { TunnelList } from "./components/TunnelList";
import { PassphraseDialog } from "./components/PassphraseDialog";
import { HostKeyDialog } from "./components/HostKeyDialog";
import { PasswordDialog } from "./components/PasswordDialog";
import { KeyboardInteractiveDialog } from "./components/KeyboardInteractiveDialog";
import { EditList } from "./components/EditList";
import { EditForm } from "./components/EditForm";
import { useTunnels } from "./hooks/useTunnels";
import type { TunnelInput } from "./types";

type View =
  | { kind: "normal" }
  | { kind: "edit-list" }
  | { kind: "edit-form"; tunnelId: string | null };

function App() {
  const {
    tunnels,
    loading,
    error,
    connect,
    disconnect,
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
  } = useTunnels();

  const [view, setView] = useState<View>({ kind: "normal" });

  const handleSave = async (input: TunnelInput, id: string | null) => {
    if (id) {
      await updateTunnel(id, input);
    } else {
      await addTunnel(input);
    }
    setView({ kind: "edit-list" });
  };

  // Edit list view
  if (view.kind === "edit-list") {
    return (
      <EditList
        tunnels={tunnels}
        onEdit={(id) => setView({ kind: "edit-form", tunnelId: id })}
        onAdd={() => setView({ kind: "edit-form", tunnelId: null })}
        onDelete={deleteTunnel}
        onDone={() => setView({ kind: "normal" })}
      />
    );
  }

  // Edit form view
  if (view.kind === "edit-form") {
    return (
      <EditForm
        tunnelId={view.tunnelId}
        tunnels={tunnels}
        getTunnelConfig={getTunnelConfig}
        onSave={handleSave}
        onBack={() => setView({ kind: "edit-list" })}
      />
    );
  }

  // Normal view
  const connectedCount = tunnels.filter((t) => t.status === "connected").length;
  const totalCount = tunnels.length;

  return (
    <div className="h-screen flex flex-col bg-[#fafafa] dark:bg-[#0f0f0f] text-[#1a1a1a] dark:text-[#e5e5e5] select-none">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)]">
        <div>
          <h1 className="text-sm font-semibold">Tunnel Master</h1>
          <p className="text-xs text-[#999] dark:text-[#666]">
            {connectedCount}/{totalCount} active
          </p>
        </div>
        <button
          onClick={() => setView({ kind: "edit-list" })}
          className="text-[#999] dark:text-[#888] hover:text-[#666] dark:hover:text-[#aaa]"
          title="Edit tunnels"
        >
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4">
            <path d="M2.695 14.763l-1.262 3.154a.5.5 0 00.65.65l3.155-1.262a4 4 0 001.343-.885L17.5 5.5a2.121 2.121 0 00-3-3L3.58 13.42a4 4 0 00-.885 1.343z" />
          </svg>
        </button>
      </div>

      {/* Error banner */}
      {error && (
        <div className="mx-3 mt-2 px-3 py-2 border-l-2 border-red-500 bg-red-500/[0.04] dark:bg-red-500/[0.06] rounded-r">
          <p className="text-xs text-[#dc2626] dark:text-[#f87171] max-h-16 overflow-y-auto break-words">{error}</p>
        </div>
      )}

      {/* Content — scrollable */}
      <div className="flex-1 overflow-y-auto p-2">
        {loading ? (
          <div className="py-8 text-center">
            <p className="text-[#999] dark:text-[#666] text-sm">Loading...</p>
          </div>
        ) : (
          <TunnelList
            tunnels={tunnels}
            onConnect={connect}
            onDisconnect={disconnect}
          />
        )}
      </div>

      {/* Footer */}
      <div className="border-t border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)] px-4 py-2">
        <button
          onClick={() => invoke("quit_app")}
          className="text-xs text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999]"
        >
          Quit Tunnel Master
        </button>
      </div>

      {/* Passphrase dialog */}
      {passphrasePrompt && (
        <PassphraseDialog
          tunnelName={passphrasePrompt.tunnelName}
          onSubmit={submitPassphrase}
          onCancel={cancelPassphrase}
        />
      )}

      {/* Host key verification dialog */}
      {hostKeyPrompt && (
        <HostKeyDialog
          host={hostKeyPrompt.host}
          port={hostKeyPrompt.port}
          keyType={hostKeyPrompt.keyType}
          fingerprint={hostKeyPrompt.fingerprint}
          isChanged={hostKeyPrompt.isChanged}
          onAccept={acceptHostKey}
          onReject={rejectHostKey}
        />
      )}

      {/* Password dialog */}
      {passwordPrompt && (
        <PasswordDialog tunnelName={passwordPrompt.tunnelName} onSubmit={submitPassword} onCancel={cancelPassword} />
      )}

      {/* Keyboard-interactive dialog */}
      {kiPrompt && (
        <KeyboardInteractiveDialog name={kiPrompt.name} instructions={kiPrompt.instructions} prompts={kiPrompt.prompts} onSubmit={respondKeyboardInteractive} onCancel={cancelKeyboardInteractive} />
      )}
    </div>
  );
}

export default App;