import { useState, useRef, useEffect } from "react";

interface CustomSelectOption {
  value: string;
  label: string;
}

interface CustomSelectProps {
  value: string | null;
  onChange: (value: string | null) => void;
  options: CustomSelectOption[];
  placeholder?: string;
}

export function CustomSelect({ value, onChange, options, placeholder = "None" }: CustomSelectProps) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  // Close on click outside
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  // Close on Escape
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [open]);

  const selectedLabel = options.find((o) => o.value === value)?.label ?? placeholder;

  return (
    <div ref={ref} className="relative flex-1">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex items-center justify-between w-full bg-transparent text-sm outline-none text-[#1a1a1a] dark:text-[#e5e5e5] cursor-pointer"
      >
        <span className={value ? "" : "text-[#bbb] dark:text-[#555]"}>{selectedLabel}</span>
        <span className="text-[#bbb] dark:text-[#555] text-xs ml-2">▾</span>
      </button>

      {open && (
        <div className="absolute top-[calc(100%+4px)] left-[-12px] right-[-12px] bg-white dark:bg-[#1a1a1a] border border-[rgba(0,0,0,0.1)] dark:border-[rgba(255,255,255,0.1)] rounded-lg shadow-lg z-20 overflow-hidden max-h-40 overflow-y-auto">
          <div
            onClick={() => { onChange(null); setOpen(false); }}
            className={`px-3 py-1.5 text-sm cursor-pointer hover:bg-[rgba(0,0,0,0.04)] dark:hover:bg-[rgba(255,255,255,0.06)] ${
              value === null ? "bg-[rgba(0,0,0,0.06)] dark:bg-[rgba(255,255,255,0.08)] font-medium" : ""
            }`}
          >
            {placeholder}
          </div>
          {options.map((opt) => (
            <div
              key={opt.value}
              onClick={() => { onChange(opt.value); setOpen(false); }}
              className={`px-3 py-1.5 text-sm cursor-pointer hover:bg-[rgba(0,0,0,0.04)] dark:hover:bg-[rgba(255,255,255,0.06)] ${
                value === opt.value ? "bg-[rgba(0,0,0,0.06)] dark:bg-[rgba(255,255,255,0.08)] font-medium" : ""
              }`}
            >
              {opt.label}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
