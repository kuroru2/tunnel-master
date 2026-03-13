import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TunnelInput, TunnelConfig, TunnelInfo, AuthMethod } from "../types";
import { CustomSelect } from "./CustomSelect";

interface EditFormProps {
  tunnelId: string | null;
  tunnels: TunnelInfo[];
  getTunnelConfig: (id: string) => Promise<TunnelConfig>;
  onSave: (input: TunnelInput, id: string | null) => Promise<void>;
  onBack: () => void;
}

const emptyForm: TunnelInput = {
  name: "", host: "", port: 22, user: "", keyPath: "",
  authMethod: "key",
  localPort: 0, remoteHost: "", remotePort: 0, autoConnect: false,
  jumpHost: null,
};

export function EditForm({ tunnelId, tunnels, getTunnelConfig, onSave, onBack }: EditFormProps) {
  const [form, setForm] = useState<TunnelInput>(emptyForm);
  const [loading, setLoading] = useState(!!tunnelId);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (tunnelId) {
      getTunnelConfig(tunnelId)
        .then((config) => {
          setForm({
            name: config.name,
            host: config.host,
            port: config.port,
            user: config.user,
            keyPath: config.keyPath,
            authMethod: config.authMethod,
            localPort: config.localPort,
            remoteHost: config.remoteHost,
            remotePort: config.remotePort,
            autoConnect: config.autoConnect,
            jumpHost: config.jumpHost,
          });
        })
        .catch((e) => setError(String(e)))
        .finally(() => setLoading(false));
    }
  }, [tunnelId, getTunnelConfig]);

  const isValid =
    form.name.trim() !== "" &&
    form.host.trim() !== "" &&
    form.user.trim() !== "" &&
    form.localPort > 0 &&
    form.remoteHost.trim() !== "" &&
    form.remotePort > 0;

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      await onSave(form, tunnelId);
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  };

  const updateField = <K extends keyof TunnelInput>(key: K, value: TunnelInput[K]) => {
    setForm((prev) => ({ ...prev, [key]: value }));
  };

  if (loading) {
    return (
      <div className="h-screen flex items-center justify-center bg-[#fafafa] dark:bg-[#0f0f0f]">
        <p className="text-[#999] dark:text-[#666] text-sm">Loading...</p>
      </div>
    );
  }

  return (
    <div className="h-screen flex flex-col bg-[#fafafa] dark:bg-[#0f0f0f] text-[#1a1a1a] dark:text-[#e5e5e5] select-none">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)]">
        <button
          onClick={onBack}
          className="text-sm text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999]"
        >
          &lsaquo; Back
        </button>
        <h1 className="text-sm font-semibold">
          {tunnelId ? "Edit Tunnel" : "New Tunnel"}
        </h1>
        <button
          onClick={handleSave}
          disabled={!isValid || saving}
          className="text-sm font-semibold disabled:text-[#bbb] dark:disabled:text-[#555] disabled:cursor-not-allowed hover:opacity-80"
        >
          {saving ? "..." : "Save"}
        </button>
      </div>

      {/* Error */}
      {error && (
        <div className="mx-3 mt-2 px-3 py-2 border-l-2 border-red-500 bg-red-500/[0.04] dark:bg-red-500/[0.06] rounded-r">
          <p className="text-xs text-[#dc2626] dark:text-[#f87171] max-h-16 overflow-y-auto break-words">{error}</p>
        </div>
      )}

      {/* Form */}
      <div className="flex-1 overflow-y-auto px-4 py-3">
        {/* Connection section */}
        <SectionLabel>Connection</SectionLabel>
        <div className="bg-[rgba(0,0,0,0.03)] dark:bg-[rgba(255,255,255,0.04)] rounded-lg overflow-hidden mb-4 border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.04)]">
          <FormRow label="Name" value={form.name} onChange={(v) => updateField("name", v)} />
          <FormRow label="Host" value={form.host} onChange={(v) => updateField("host", v)} />
          <FormRow
            label="Port"
            value={String(form.port)}
            onChange={(v) => updateField("port", parseInt(v) || 0)}
            type="number"
            mono
          />
          <FormRow label="Username" value={form.user} onChange={(v) => updateField("user", v)} />

          {/* Auth Method */}
          <div className="flex items-center px-3 py-2 border-b border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.04)]">
            <label className="text-sm text-[#999] dark:text-[#666] w-[70px] flex-shrink-0">Auth</label>
            <div className="flex gap-1 flex-1">
              {(["key", "password", "agent", "keyboard-interactive"] as AuthMethod[]).map((method) => {
                const labels: Record<AuthMethod, string> = { key: "Key", password: "Password", agent: "Agent", "keyboard-interactive": "2FA" };
                return (
                  <button key={method} type="button" onClick={() => updateField("authMethod", method)}
                    className={`px-2 py-1 text-xs rounded-md transition-colors ${
                      form.authMethod === method
                        ? "bg-[#1a1a1a] dark:bg-[#e5e5e5] text-white dark:text-[#0f0f0f] font-medium"
                        : "text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999]"
                    }`}>
                    {labels[method]}
                  </button>
                );
              })}
            </div>
          </div>

          {form.authMethod === "key" && (
            <div className="flex items-center px-3 py-2">
              <label className="text-sm text-[#999] dark:text-[#666] w-[70px] flex-shrink-0">Key</label>
              <input
                type="text"
                value={form.keyPath}
                onChange={(e) => updateField("keyPath", e.target.value)}
                placeholder="~/.ssh/id_rsa"
                className="flex-1 bg-transparent text-sm outline-none placeholder-[#bbb] dark:placeholder-[#555]"
                style={{ fontFamily: "var(--font-mono)" }}
              />
              <button
                type="button"
                onClick={async () => {
                  try {
                    const path = await invoke<string | null>("pick_key_file");
                    if (path) updateField("keyPath", path);
                  } catch (e) {
                    setError(String(e));
                  }
                }}
                className="ml-2 text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999] flex-shrink-0"
                title="Browse for key file"
              >
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4">
                  <path fillRule="evenodd" d="M3.75 3A1.75 1.75 0 002 4.75v3.26a3.235 3.235 0 011.75-.51h12.5c.644 0 1.245.188 1.75.51V6.75A1.75 1.75 0 0016.25 5h-4.836a.25.25 0 01-.177-.073L9.823 3.513A1.75 1.75 0 008.586 3H3.75zM3.75 9A1.75 1.75 0 002 10.75v4.5c0 .966.784 1.75 1.75 1.75h12.5A1.75 1.75 0 0018 15.25v-4.5A1.75 1.75 0 0016.25 9H3.75z" clipRule="evenodd" />
                </svg>
              </button>
            </div>
          )}

          {/* Jump Host */}
          <div className="flex items-center px-3 py-2 border-t border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.04)]">
            <label className="text-sm text-[#999] dark:text-[#666] w-[70px] flex-shrink-0">Jump</label>
            <CustomSelect
              value={form.jumpHost}
              onChange={(v) => updateField("jumpHost", v)}
              options={tunnels.filter((t) => t.id !== tunnelId).map((t) => ({ value: t.id, label: t.name }))}
            />
          </div>
        </div>

        {/* Port Forwarding section */}
        <SectionLabel>Port Forwarding</SectionLabel>
        <div className="bg-[rgba(0,0,0,0.03)] dark:bg-[rgba(255,255,255,0.04)] rounded-lg overflow-hidden mb-4 border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.04)]">
          <FormRow
            label="Local"
            value={form.localPort === 0 ? "" : String(form.localPort)}
            onChange={(v) => updateField("localPort", parseInt(v) || 0)}
            type="number"
            placeholder="e.g. 5432"
            mono
          />
          <FormRow
            label="Remote"
            value={form.remoteHost}
            onChange={(v) => updateField("remoteHost", v)}
            placeholder="e.g. localhost"
            mono
          />
          <FormRow
            label="Port"
            value={form.remotePort === 0 ? "" : String(form.remotePort)}
            onChange={(v) => updateField("remotePort", parseInt(v) || 0)}
            type="number"
            placeholder="e.g. 5432"
            mono
          />
        </div>

        {/* Options section */}
        <SectionLabel>Options</SectionLabel>
        <div className="bg-[rgba(0,0,0,0.03)] dark:bg-[rgba(255,255,255,0.04)] rounded-lg overflow-hidden border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.04)]">
          <div className="flex items-center justify-between px-3 py-2.5">
            <span className="text-sm text-[#999] dark:text-[#999]">Auto Connect</span>
            <button
              onClick={() => updateField("autoConnect", !form.autoConnect)}
              className={`w-8 h-[18px] rounded-full relative transition-colors ${
                form.autoConnect ? "bg-[#4ade80]" : "bg-[#ccc] dark:bg-[#333]"
              }`}
            >
              <div
                className={`w-[14px] h-[14px] rounded-full absolute top-[2px] transition-transform ${
                  form.autoConnect
                    ? "translate-x-[14px] bg-white"
                    : "translate-x-[2px] bg-white dark:bg-[#888]"
                }`}
              />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <p className="text-xs text-[#bbb] dark:text-[#555] uppercase tracking-wider mb-1.5 px-1">
      {children}
    </p>
  );
}

interface FormRowProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: string;
  placeholder?: string;
  mono?: boolean;
}

function FormRow({ label, value, onChange, type = "text", placeholder, mono }: FormRowProps) {
  return (
    <div className="flex items-center px-3 py-2 border-b border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.04)] last:border-b-0">
      <label className="text-sm text-[#999] dark:text-[#666] w-[70px] flex-shrink-0">{label}</label>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="flex-1 bg-transparent text-sm outline-none placeholder-[#bbb] dark:placeholder-[#555]"
        style={mono ? { fontFamily: "var(--font-mono)" } : undefined}
      />
    </div>
  );
}