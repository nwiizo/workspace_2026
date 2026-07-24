import { FC } from "react";
import { twMerge } from "tailwind-merge";

type Props = {
  checked: boolean;
  id: string;
  onUpdate: (v: boolean) => void;
  disabled?: boolean;
  className?: string;
};

export const Toggle: FC<Props> = ({
  onUpdate,
  className,
  checked,
  id,
  disabled,
}) => {
  return (
    <div className={twMerge("relative inline-block w-14 h-7", className)}>
      <input
        checked={checked}
        onChange={() => onUpdate(!checked)}
        id={id}
        type="checkbox"
        className={twMerge(
          "peer appearance-none w-14 h-7",
          "bg-slate-100 rounded-full checked:bg-emerald-500",
          "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-gray-900",
          "transition-colors duration-300",
          !disabled && "cursor-pointer",
        )}
        disabled={disabled}
      />
      {/* eslint-disable-next-line jsx-a11y/label-has-associated-control */}
      <label
        htmlFor={id}
        className={twMerge(
          "absolute top-0 left-0 w-7 h-7 pointer-events-none",
          "bg-white rounded-full border border-slate-300 shadow-sm",
          "transition-transform duration-300",
          "peer-checked:translate-x-7 peer-checked:border-emerald-500",
          !disabled && "cursor-pointer",
        )}
      />
    </div>
  );
};
