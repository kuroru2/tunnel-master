import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TunnelInput, TunnelConfig } from "../types";

interface EditFormProps {
  tunnelId: string | null; // null = new tunnel
  getTunnelConfig: (id: string) => Promise<TunnelConfig>;
  onSave: (input: TunnelInput, id: string | null) => Promise<void>;
  onBack: () => void;
}

const emptyForm: TunnelInput = {
  name: "",
  host: "",
  port: 22,
  user: "",
  keyPath: "",
  localPort: 0,
  remoteHost: "",
  remotePort: 0,
  autoConnect: false,
};

export function EditForm({ tunnelId, getTunnelConfig, onSave, onBack }: EditFormProps) {
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
            localPort: config.localPort,
            remoteHost: config.remoteHost,
            remotePort: config.remotePort,
            autoConnect: config.autoConnect,
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
      <div className="h-screen flex items-center justify-center bg-gray-900">
        <p className="text-gray-400 text-sm">Loading...</p>
      </div>
    );
  }

  return (
    <div className="h-screen flex flex-col bg-gray-900 text-white select-none">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
        <button
          onClick={onBack}
          className="text-sm text-blue-400 hover:text-blue-300"
        >
          &lsaquo; Back
        </button>
        <h1 className="text-sm font-semibold">
          {tunnelId ? "Edit Tunnel" : "New Tunnel"}
        </h1>
        <button
          onClick={handleSave}
          disabled={!isValid || saving}
          className="text-sm font-semibold text-blue-400 hover:text-blue-300 disabled:text-gray-600 disabled:cursor-not-allowed"
        >
          {saving ? "..." : "Save"}
        </button>
      </div>

      {/* Error */}
      {error && (
        <div className="mx-3 mt-2 px-3 py-2 bg-red-500/20 border border-red-500/30 rounded-md">
          <p className="text-xs text-red-400">{error}</p>
        </div>
      )}

      {/* Form */}
      <div className="flex-1 overflow-y-auto px-4 py-3">
        {/* Connection section */}
        <SectionLabel>Connection</SectionLabel>
        <div className="bg-white/5 rounded-lg overflow-hidden mb-4">
          <FormRow label="Name" value={form.name} onChange={(v) => updateField("name", v)} />
          <FormRow label="Host" value={form.host} onChange={(v) => updateField("host", v)} />
          <FormRow
            label="Port"
            value={String(form.port)}
            onChange={(v) => updateField("port", parseInt(v) || 0)}
            type="number"
          />
          <FormRow label="Username" value={form.user} onChange={(v) => updateField("user", v)} />
          <div className="flex items-center px-3 py-2">
            <label className="text-sm text-gray-400 w-24 flex-shrink-0">Key Path</label>
            <input
              type="text"
              value={form.keyPath}
              onChange={(e) => updateField("keyPath", e.target.value)}
              placeholder="~/.ssh/id_rsa"
              className="flex-1 bg-transparent text-sm text-white outline-none placeholder-gray-600"
            />
            <button
              type="button"
              onClick={async () => {
                const path = await invoke<string | null>("pick_key_file");
                if (path) updateField("keyPath", path);
              }}
              className="ml-2 text-blue-400 hover:text-blue-300 flex-shrink-0"
              title="Browse for key file"
            >
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4">
                <path fillRule="evenodd" d="M3.75 3A1.75 1.75 0 002 4.75v3.26a3.235 3.235 0 011.75-.51h12.5c.644 0 1.245.188 1.75.51V6.75A1.75 1.75 0 0016.25 5h-4.836a.25.25 0 01-.177-.073L9.823 3.513A1.75 1.75 0 008.586 3H3.75zM3.75 9A1.75 1.75 0 002 10.75v4.5c0 .966.784 1.75 1.75 1.75h12.5A1.75 1.75 0 0018 15.25v-4.5A1.75 1.75 0 0016.25 9H3.75z" clipRule="evenodd" />
              </svg>
            </button>
          </div>
        </div>

        {/* Port Forwarding section */}
        <SectionLabel>Port Forwarding</SectionLabel>
        <div className="bg-white/5 rounded-lg overflow-hidden mb-4">
          <FormRow
            label="Local Port"
            value={form.localPort === 0 ? "" : String(form.localPort)}
            onChange={(v) => updateField("localPort", parseInt(v) || 0)}
            type="number"
            placeholder="e.g. 5432"
          />
          <FormRow
            label="Remote Host"
            value={form.remoteHost}
            onChange={(v) => updateField("remoteHost", v)}
            placeholder="e.g. localhost"
          />
          <FormRow
            label="Remote Port"
            value={form.remotePort === 0 ? "" : String(form.remotePort)}
            onChange={(v) => updateField("remotePort", parseInt(v) || 0)}
            type="number"
            placeholder="e.g. 5432"
            last
          />
        </div>

        {/* Options section */}
        <SectionLabel>Options</SectionLabel>
        <div className="bg-white/5 rounded-lg overflow-hidden">
          <div className="flex items-center justify-between px-3 py-2.5">
            <span className="text-sm text-gray-300">Auto Connect</span>
            <button
              onClick={() => updateField("autoConnect", !form.autoConnect)}
              className={`w-10 h-6 rounded-full relative transition-colors ${
                form.autoConnect ? "bg-green-500" : "bg-gray-600"
              }`}
            >
              <div
                className={`w-5 h-5 rounded-full bg-white absolute top-0.5 transition-transform ${
                  form.autoConnect ? "translate-x-[18px]" : "translate-x-0.5"
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
    <p className="text-xs text-gray-500 uppercase tracking-wider mb-1.5 px-1">
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
  last?: boolean;
}

function FormRow({ label, value, onChange, type = "text", placeholder, last }: FormRowProps) {
  return (
    <div
      className={`flex items-center px-3 py-2 ${
        last ? "" : "border-b border-white/5"
      }`}
    >
      <label className="text-sm text-gray-400 w-24 flex-shrink-0">{label}</label>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="flex-1 bg-transparent text-sm text-white outline-none placeholder-gray-600"
      />
    </div>
  );
}
