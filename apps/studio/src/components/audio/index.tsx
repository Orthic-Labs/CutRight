// apps/studio/src/components/audio/index.tsx — CR-V2-B6-014.
export function TransientMarker(props: { transient_id: string; position_ms: number }) {
  return <span data-transient-id={props.transient_id} className="transient-marker">{props.position_ms}ms</span>;
}
