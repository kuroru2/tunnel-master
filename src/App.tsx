import { useState } from "react";
import { TunnelList } from "./components/TunnelList";
import { PassphraseDialog } from "./components/PassphraseDialog";
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
    <div className="h-screen flex flex-col bg-gray-900 text-white select-none">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
        <div>
          <h1 className="text-sm font-semibold">Tunnel Master</h1>
          <p className="text-xs text-gray-400">
            {connectedCount}/{totalCount} active
          </p>
        </div>
        <button
          onClick={() => setView({ kind: "edit-list" })}
          className="text-sm text-blue-400 hover:text-blue-300"
        >
          Edit
        </button>
      </div>

      {/* Error banner */}
      {error && (
        <div className="mx-3 mt-2 px-3 py-2 bg-red-500/20 border border-red-500/30 rounded-md">
          <p className="text-xs text-red-400">{error}</p>
        </div>
      )}

      {/* Content — scrollable */}
      <div className="flex-1 overflow-y-auto p-2">
        {loading ? (
          <div className="py-8 text-center">
            <p className="text-gray-400 text-sm">Loading...</p>
          </div>
        ) : (
          <TunnelList
            tunnels={tunnels}
            onConnect={connect}
            onDisconnect={disconnect}
          />
        )}
      </div>

      {/* Passphrase dialog */}
      {passphrasePrompt && (
        <PassphraseDialog
          tunnelId={passphrasePrompt.tunnelId}
          onSubmit={submitPassphrase}
          onCancel={cancelPassphrase}
        />
      )}
    </div>
  );
}

export default App;
