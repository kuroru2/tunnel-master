import { useState, useRef, useCallback } from "react";
import type { TunnelInfo } from "../types";

interface EditListProps {
  tunnels: TunnelInfo[];
  onEdit: (id: string) => void;
  onAdd: () => void;
  onDelete: (id: string) => Promise<void>;
  onReorder: (ids: string[]) => Promise<void>;
  onDone: () => void;
}

export function EditList({ tunnels, onEdit, onAdd, onDelete, onReorder, onDone }: EditListProps) {
  const [confirmingDelete, setConfirmingDelete] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [dragState, setDragState] = useState<{ srcIndex: number; overIndex: number } | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const rowRectsRef = useRef<DOMRect[]>([]);

  const handleMinusClick = (id: string) => {
    setConfirmingDelete(confirmingDelete === id ? null : id);
  };

  const handleDelete = async (id: string) => {
    setDeleting(id);
    try {
      await onDelete(id);
    } finally {
      setDeleting(null);
      setConfirmingDelete(null);
    }
  };

  const getOverIndex = useCallback((clientY: number): number => {
    const rects = rowRectsRef.current;
    for (let i = 0; i < rects.length; i++) {
      const mid = rects[i].top + rects[i].height / 2;
      if (clientY < mid) return i;
    }
    return rects.length - 1;
  }, []);

  const handlePointerDown = useCallback((e: React.PointerEvent, index: number) => {
    e.preventDefault();
    e.stopPropagation();

    // Snapshot row positions before drag starts
    if (listRef.current) {
      const rows = listRef.current.querySelectorAll("[data-drag-row]");
      rowRectsRef.current = Array.from(rows).map((r) => r.getBoundingClientRect());
    }

    setDragState({ srcIndex: index, overIndex: index });

    const handleMove = (ev: PointerEvent) => {
      const over = getOverIndex(ev.clientY);
      setDragState((prev) => prev ? { ...prev, overIndex: over } : null);
    };

    const handleUp = async (ev: PointerEvent) => {
      document.removeEventListener("pointermove", handleMove);
      document.removeEventListener("pointerup", handleUp);

      const over = getOverIndex(ev.clientY);
      setDragState(null);

      if (over !== index) {
        const ids = tunnels.map((t) => t.id);
        const [moved] = ids.splice(index, 1);
        ids.splice(over, 0, moved);
        await onReorder(ids);
      }
    };

    document.addEventListener("pointermove", handleMove);
    document.addEventListener("pointerup", handleUp);
  }, [tunnels, onReorder, getOverIndex]);

  return (
    <div className="h-screen flex flex-col bg-[#fafafa] dark:bg-[#0f0f0f] text-[#1a1a1a] dark:text-[#e5e5e5] select-none">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)]">
        <button
          onClick={onAdd}
          className="text-sm text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999]"
        >
          + Add
        </button>
        <h1 className="text-sm font-semibold">Edit Tunnels</h1>
        <button
          onClick={onDone}
          className="text-sm font-semibold hover:opacity-80"
        >
          Done
        </button>
      </div>

      {/* Tunnel list */}
      <div className="flex-1 overflow-y-auto overflow-x-hidden p-2">
        {tunnels.length === 0 ? (
          <div className="py-8 text-center">
            <p className="text-[#999] dark:text-[#666] text-sm">No tunnels configured</p>
            <button
              onClick={onAdd}
              className="mt-2 text-[#999] dark:text-[#666] text-sm hover:text-[#666] dark:hover:text-[#999]"
            >
              Add your first tunnel
            </button>
          </div>
        ) : (
          <div ref={listRef} className="space-y-0.5">
            {tunnels.map((tunnel, index) => (
              <div
                key={tunnel.id}
                data-drag-row
                className={`flex items-center rounded-lg hover:bg-[rgba(0,0,0,0.03)] dark:hover:bg-[rgba(255,255,255,0.04)] transition-all min-w-0 ${
                  dragState?.srcIndex === index ? "opacity-40" : ""
                } ${
                  dragState && dragState.overIndex === index && dragState.srcIndex !== index
                    ? "border-t-2 border-[#4ade80]"
                    : "border-t-2 border-transparent"
                }`}
              >
                {/* Drag handle — pointer events only */}
                <div
                  onPointerDown={(e) => handlePointerDown(e, index)}
                  className="flex-shrink-0 w-6 flex items-center justify-center cursor-grab active:cursor-grabbing text-[#ccc] dark:text-[#444] hover:text-[#999] dark:hover:text-[#666] touch-none"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" className="w-3.5 h-3.5">
                    <circle cx="5.5" cy="3.5" r="1" />
                    <circle cx="10.5" cy="3.5" r="1" />
                    <circle cx="5.5" cy="8" r="1" />
                    <circle cx="10.5" cy="8" r="1" />
                    <circle cx="5.5" cy="12.5" r="1" />
                    <circle cx="10.5" cy="12.5" r="1" />
                  </svg>
                </div>

                {/* Delete minus button */}
                <button
                  onClick={() => handleMinusClick(tunnel.id)}
                  className="flex-shrink-0 w-8 h-8 flex items-center justify-center"
                  disabled={deleting === tunnel.id}
                >
                  <div className="w-5 h-5 rounded-full bg-[#dc2626] flex items-center justify-center text-white text-sm font-bold leading-none">
                    &minus;
                  </div>
                </button>

                {/* Tunnel info — clickable to edit */}
                <button
                  onClick={() => onEdit(tunnel.id)}
                  className="flex-1 flex items-center justify-between px-2 py-2.5 text-left min-w-0 overflow-hidden"
                >
                  <div className="min-w-0">
                    <p className="text-sm truncate">{tunnel.name}</p>
                    <p className="text-xs text-[#999] dark:text-[#555] truncate" style={{ fontFamily: "var(--font-mono)" }}>
                      localhost:{tunnel.localPort} &rarr; {tunnel.remoteHost}:{tunnel.remotePort}
                    </p>
                  </div>
                  <span className="text-[#ccc] dark:text-[#333] text-lg ml-2">&rsaquo;</span>
                </button>

                {/* Slide-in delete confirmation */}
                {confirmingDelete === tunnel.id && (
                  <button
                    onClick={() => handleDelete(tunnel.id)}
                    disabled={deleting === tunnel.id}
                    className="flex-shrink-0 bg-red-500 text-white text-xs px-3 py-1.5 rounded-md mr-2 hover:bg-red-600 disabled:opacity-50"
                  >
                    {deleting === tunnel.id ? "..." : "Delete"}
                  </button>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
