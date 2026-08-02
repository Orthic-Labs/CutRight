import { REGISTER_LABEL, REGISTER_ORDER, type Register } from "../types";

// QA-only register switcher (redesign spec Phase 2: "PARKED for Adrian's
// eyes as rendered screenshots"). Only ever mounted behind `?qa=1` — see
// App.tsx — so it never ships as a real in-app control before Adrian picks
// a register and it gets locked into brands.md. Exists purely so this pass
// can screenshot all three registers from one running instance instead of
// three separate builds.
export function RegisterSwitch({
  register,
  setRegister,
}: {
  register: Register;
  setRegister: (value: Register) => void;
}) {
  return (
    <div
      className="register-switch"
      role="radiogroup"
      aria-label="QA: register"
    >
      {REGISTER_ORDER.map((item) => (
        <button
          key={item}
          role="radio"
          data-register-option={item}
          className={register === item ? "active" : ""}
          aria-checked={register === item}
          onClick={() => setRegister(item)}
        >
          {REGISTER_LABEL[item]}
        </button>
      ))}
    </div>
  );
}
