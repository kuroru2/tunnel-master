import { useState } from "react";

interface HostKeyDialogProps {
  host: string;
  port: number;
  keyType: string;
  fingerprint: string;
  isChanged: boolean;
  onAccept: () => void;
  onReject: () => void;
}

export function HostKeyDialog({
  host,
  port,
  keyType,
  fingerprint,
  isChanged,
  onAccept,
  onReject,
}: HostKeyDialogProps) {
  const [accepting, setAccepting] = useState(false);

  const handleAccept = async () => {
    setAccepting(true);
    onAccept();
  };

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50">
      <div className="bg-white dark:bg-[#1a1a1a] rounded-xl p-4 mx-3 w-full border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.08)]">
        {isChanged ? (
          <>
            <h3 className="text-sm font-semibold text-[#dc2626] dark:text-[#f87171] mb-1">
              Host Key Changed
            </h3>
            <p className="text-xs text-[#dc2626] dark:text-[#f87171] mb-3">
              The host key for <span className="font-semibold">{host}:{port}</span> has changed.
              This could indicate a man-in-the-middle attack. Connection refused.
            </p>
            <div className="px-3 py-2 bg-[#fafafa] dark:bg-[#0f0f0f] border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)] rounded-md mb-3">
              <p className="text-xs text-[#999] dark:text-[#666]">
                If this is expected (e.g. server was reinstalled), remove the old entry from{" "}
                <span style={{ fontFamily: "var(--font-mono)" }}>~/.ssh/known_hosts</span> and try again.
              </p>
            </div>
            <div className="flex justify-end">
              <button
                onClick={onReject}
                className="px-3 py-1.5 text-xs font-medium bg-[#1a1a1a] dark:bg-[#e5e5e5] text-[#fafafa] dark:text-[#0f0f0f] rounded-md hover:opacity-90"
              >
                OK
              </button>
            </div>
          </>
        ) : (
          <>
            <h3 className="text-sm font-semibold mb-1">Unknown Host</h3>
            <p className="text-xs text-[#999] dark:text-[#666] mb-3">
              The authenticity of <span className="text-[#1a1a1a] dark:text-[#e5e5e5]">{host}:{port}</span> can't be established.
            </p>
            <div className="px-3 py-2 bg-[#fafafa] dark:bg-[#0f0f0f] border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)] rounded-md mb-3">
              <p className="text-xs text-[#999] dark:text-[#666] mb-0.5">{keyType} fingerprint:</p>
              <p className="text-xs break-all" style={{ fontFamily: "var(--font-mono)" }}>
                SHA256:{fingerprint}
              </p>
            </div>
            <p className="text-xs text-[#999] dark:text-[#666] mb-3">
              Are you sure you want to continue connecting?
            </p>
            <div className="flex gap-2 justify-end">
              <button
                onClick={onReject}
                className="px-3 py-1.5 text-xs text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999] rounded"
              >
                Cancel
              </button>
              <button
                onClick={handleAccept}
                disabled={accepting}
                className="px-3 py-1.5 text-xs font-medium bg-[#1a1a1a] dark:bg-[#e5e5e5] text-[#fafafa] dark:text-[#0f0f0f] rounded-md hover:opacity-90 disabled:opacity-50"
              >
                {accepting ? "..." : "Trust & Connect"}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
