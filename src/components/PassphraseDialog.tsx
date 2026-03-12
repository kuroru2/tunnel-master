import { useState } from "react";

interface PassphraseDialogProps {
  tunnelName: string;
  onSubmit: (passphrase: string) => void;
  onCancel: () => void;
}

export function PassphraseDialog({
  tunnelName,
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
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50">
      <form
        onSubmit={handleSubmit}
        className="bg-white dark:bg-[#1a1a1a] rounded-xl p-4 mx-3 w-full border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.08)]"
      >
        <h3 className="text-sm font-semibold mb-1">Passphrase Required</h3>
        <p className="text-xs text-[#999] dark:text-[#666] mb-3">
          Enter the passphrase for <span className="text-[#1a1a1a] dark:text-[#e5e5e5]">{tunnelName}</span>'s SSH key.
          It will be stored securely.
        </p>
        <input
          type="password"
          value={passphrase}
          onChange={(e) => setPassphrase(e.target.value)}
          placeholder="SSH key passphrase"
          autoFocus
          className="w-full px-3 py-2 bg-[#fafafa] dark:bg-[#0f0f0f] border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)] rounded-md text-sm placeholder-[#bbb] dark:placeholder-[#555] focus:outline-none focus:ring-1 focus:ring-[#bbb] dark:focus:ring-[#555] mb-3"
        />
        <div className="flex gap-2 justify-end">
          <button
            type="button"
            onClick={onCancel}
            className="px-3 py-1.5 text-xs text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999] rounded"
          >
            Cancel
          </button>
          <button
            type="submit"
            className="px-3 py-1.5 text-xs font-medium bg-[#1a1a1a] dark:bg-[#e5e5e5] text-[#fafafa] dark:text-[#0f0f0f] rounded-md hover:opacity-90"
          >
            Unlock
          </button>
        </div>
      </form>
    </div>
  );
}
