import { useState } from "react";

interface PasswordDialogProps {
  tunnelName: string;
  onSubmit: (password: string) => void;
  onCancel: () => void;
}

export function PasswordDialog({ tunnelName, onSubmit, onCancel }: PasswordDialogProps) {
  const [password, setPassword] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (password) onSubmit(password);
  };

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50">
      <form onSubmit={handleSubmit} className="bg-white dark:bg-[#1a1a1a] rounded-xl p-4 mx-3 w-full border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.08)]">
        <h3 className="text-sm font-semibold mb-1">Password Required</h3>
        <p className="text-xs text-[#999] dark:text-[#666] mb-3">
          Enter the SSH password for{" "}
          <span className="text-[#1a1a1a] dark:text-[#e5e5e5]">{tunnelName}</span>. It will be stored securely.
        </p>
        <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="SSH password" autoFocus className="w-full px-3 py-2 bg-[#fafafa] dark:bg-[#0f0f0f] border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)] rounded-md text-sm placeholder-[#bbb] dark:placeholder-[#555] focus:outline-none focus:ring-1 focus:ring-[#bbb] dark:focus:ring-[#555] mb-3" />
        <div className="flex gap-2 justify-end">
          <button type="button" onClick={onCancel} className="px-3 py-1.5 text-xs text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999] rounded">Cancel</button>
          <button type="submit" className="px-3 py-1.5 text-xs font-medium bg-[#1a1a1a] dark:bg-[#e5e5e5] text-[#fafafa] dark:text-[#0f0f0f] rounded-md hover:opacity-90">Connect</button>
        </div>
      </form>
    </div>
  );
}