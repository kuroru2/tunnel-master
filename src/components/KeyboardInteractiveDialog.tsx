import { useState } from "react";

interface KeyboardInteractiveDialogProps {
  name: string;
  instructions: string;
  prompts: Array<{ text: string; echo: boolean }>;
  onSubmit: (responses: string[]) => void;
  onCancel: () => void;
}

export function KeyboardInteractiveDialog({ name, instructions, prompts, onSubmit, onCancel }: KeyboardInteractiveDialogProps) {
  const [responses, setResponses] = useState<string[]>(prompts.map(() => ""));

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(responses);
  };

  const updateResponse = (index: number, value: string) => {
    setResponses((prev) => {
      const next = [...prev];
      next[index] = value;
      return next;
    });
  };

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50">
      <form onSubmit={handleSubmit} className="bg-white dark:bg-[#1a1a1a] rounded-xl p-4 mx-3 w-full border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.08)]">
        <h3 className="text-sm font-semibold mb-1">{name || "Authentication Required"}</h3>
        {instructions && <p className="text-xs text-[#999] dark:text-[#666] mb-3">{instructions}</p>}
        {prompts.map((prompt, i) => (
          <div key={i} className="mb-3">
            <label className="text-xs text-[#999] dark:text-[#666] mb-1 block">{prompt.text}</label>
            <input type={prompt.echo ? "text" : "password"} value={responses[i]} onChange={(e) => updateResponse(i, e.target.value)} autoFocus={i === 0} className="w-full px-3 py-2 bg-[#fafafa] dark:bg-[#0f0f0f] border border-[rgba(0,0,0,0.06)] dark:border-[rgba(255,255,255,0.06)] rounded-md text-sm placeholder-[#bbb] dark:placeholder-[#555] focus:outline-none focus:ring-1 focus:ring-[#bbb] dark:focus:ring-[#555]" />
          </div>
        ))}
        <div className="flex gap-2 justify-end">
          <button type="button" onClick={onCancel} className="px-3 py-1.5 text-xs text-[#999] dark:text-[#666] hover:text-[#666] dark:hover:text-[#999] rounded">Cancel</button>
          <button type="submit" className="px-3 py-1.5 text-xs font-medium bg-[#1a1a1a] dark:bg-[#e5e5e5] text-[#fafafa] dark:text-[#0f0f0f] rounded-md hover:opacity-90">Submit</button>
        </div>
      </form>
    </div>
  );
}