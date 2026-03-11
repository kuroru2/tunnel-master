import { TunnelList } from "./components/TunnelList";
import { PassphraseDialog } from "./components/PassphraseDialog";
import { useTunnels } from "./hooks/useTunnels";

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
  } = useTunnels();

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
