import { useEffect, useRef, useState, type CSSProperties } from "react";

interface NameInputProps {
  initialValue?: string;
  onConfirm: (value: string) => void;
  onCancel: () => void;
  style?: CSSProperties;
}

export function NameInput({ initialValue = "", onConfirm, onCancel, style }: NameInputProps) {
  const [value, setValue] = useState(initialValue);
  const inputRef = useRef<HTMLInputElement>(null);
  const settledRef = useRef(false);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  const settle = (commit: boolean) => {
    if (settledRef.current) return;
    settledRef.current = true;
    if (commit) onConfirm(value);
    else onCancel();
  };

  return (
    <input
      ref={inputRef}
      className="tree-item__name-input"
      style={style}
      value={value}
      onChange={(e) => setValue(e.target.value)}
      onBlur={() => settle(true)}
      onKeyDown={(e) => {
        if (e.key === "Enter") settle(true);
        if (e.key === "Escape") settle(false);
      }}
    />
  );
}
