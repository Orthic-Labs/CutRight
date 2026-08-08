import { useEffect, useMemo, useRef, useState } from "react";

export type FinishVariant = {
  id: string;
  label?: string;
  previewUrl?: string | null;
  sourceHashes: readonly string[];
  score?: number;
};

type Props = {
  variants: readonly FinishVariant[];
  lockedCutHash: string;
  currentCutHash: string;
  selectedId?: string | null;
  onCommit: (variant: FinishVariant, lockedCutHash: string) => Promise<void> | void;
  onSeek?: (seconds: number) => void;
};

/** Deterministic 4–5 choice surface. Pixels remain source artifacts; only ID/hash is committed. */
export function VariantAudition({ variants, lockedCutHash, currentCutHash, selectedId, onCommit, onSeek }: Props) {
  const choices = useMemo(() => variants.slice(0, 5), [variants]);
  const [focus, setFocus] = useState(Math.max(0, choices.findIndex((v) => v.id === selectedId)));
  const [pending, setPending] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  useEffect(() => { if (focus >= choices.length) setFocus(Math.max(0, choices.length - 1)); }, [choices.length, focus]);
  if (choices.length < 4 || choices.length > 5) return <section aria-label="Finish variants">Need 4–5 variants.</section>;
  const choose = async (variant: FinishVariant) => {
    if (currentCutHash !== lockedCutHash) { setMessage("Cut changed; refresh audition before committing."); return; }
    setPending(true); setMessage(null);
    try { await onCommit(variant, lockedCutHash); setMessage(`Selected ${variant.label ?? variant.id}`); }
    catch (error) { setMessage(error instanceof Error ? error.message : "Selection failed"); }
    finally { setPending(false); }
  };
  return <section className="variant-audition" aria-label="Finish variant audition">
    <header><h2>Finish audition</h2><p>{choices.length} deterministic options · locked cut</p></header>
    <div ref={listRef} role="listbox" aria-label="Finish variants" aria-activedescendant={choices[focus]?.id}
      tabIndex={0} onKeyDown={(event) => {
        if (event.key === "ArrowRight" || event.key === "ArrowDown") { event.preventDefault(); setFocus((focus + 1) % choices.length); }
        if (event.key === "ArrowLeft" || event.key === "ArrowUp") { event.preventDefault(); setFocus((focus + choices.length - 1) % choices.length); }
        if (event.key === "Enter" && choices[focus]) { event.preventDefault(); void choose(choices[focus]); }
        if (event.key === " ") { event.preventDefault(); onSeek?.(0); }
      }}>
      {choices.map((variant, index) => <article key={variant.id} id={variant.id} role="option" aria-selected={variant.id === selectedId} className={index === focus ? "is-focused" : ""}>
        {variant.previewUrl ? <video src={variant.previewUrl} controls preload="metadata" aria-label={`${variant.label ?? variant.id} preview`} onTimeUpdate={(e) => onSeek?.(e.currentTarget.currentTime)} /> : <div className="variant-placeholder" aria-label="Preview unavailable">Preview pending</div>}
        <h3>{variant.label ?? variant.id}</h3><small>{variant.score == null ? "" : `score ${variant.score.toFixed(2)}`}</small>
        <button type="button" disabled={pending} onFocus={() => setFocus(index)} onClick={() => void choose(variant)}>{variant.id === selectedId ? "Selected" : "Choose"}</button>
      </article>)}
    </div>
    <p role="status" aria-live="polite">{message}</p>
  </section>;
}
