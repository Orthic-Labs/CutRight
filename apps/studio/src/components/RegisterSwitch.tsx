import { REGISTER_LABEL, REGISTER_ORDER, type Register } from "../types";

// QA-only register switcher. Graphite is locked for production; Tungsten and
// Pewter remain capture-only regression themes. This control is mounted only
// behind `?qa=1` and never becomes a user-facing identity picker.
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
