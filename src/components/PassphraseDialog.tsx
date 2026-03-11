import { useState } from "react";

interface PassphraseDialogProps {
  tunnelId: string;
  onSubmit: (passphrase: string) => void;
  onCancel: () => void;
}

export function PassphraseDialog({
  tunnelId,
  onSubmit,
  onCancel,
}: PassphraseDialogProps) {
  const [passphrase, setPassphrase] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (passphrase.trim()) {
      onSubmit(passphrase);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <form
        onSubmit={handleSubmit}
        className="bg-gray-800 rounded-lg p-4 mx-3 w-full border border-white/10"
      >
        <h3 className="text-sm font-semibold mb-1">Passphrase Required</h3>
        <p className="text-xs text-gray-400 mb-3">
          Enter the passphrase for <span className="text-gray-300">{tunnelId}</span>'s SSH key.
          It will be stored in your macOS Keychain.
        </p>
        <input
          type="password"
          value={passphrase}
          onChange={(e) => setPassphrase(e.target.value)}
          placeholder="SSH key passphrase"
          autoFocus
          className="w-full px-3 py-2 bg-gray-900 border border-white/10 rounded text-sm text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 mb-3"
        />
        <div className="flex gap-2 justify-end">
          <button
            type="button"
            onClick={onCancel}
            className="px-3 py-1.5 text-xs text-gray-400 hover:text-white rounded"
          >
            Cancel
          </button>
          <button
            type="submit"
            className="px-3 py-1.5 text-xs bg-blue-600 hover:bg-blue-700 text-white rounded"
          >
            Unlock & Connect
          </button>
        </div>
      </form>
    </div>
  );
}
