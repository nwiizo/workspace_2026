import {
  ChangeEventHandler,
  ComponentProps,
  FC,
  PropsWithoutRef,
  useState,
} from "react";
import { twMerge } from "tailwind-merge";

type TextInputProps = PropsWithoutRef<{
  id: string;
  label: string;
  name: string;
  className?: string;
}> &
  ComponentProps<"input">;

export const TextInput: FC<TextInputProps> = ({
  value: valueFromProps,
  onChange: onChangeFromProps,
  defaultValue,
  id,
  className,
  label,
  ...props
}) => {
  const isControlled = typeof valueFromProps != "undefined";
  const hasDefaultValue = typeof defaultValue != "undefined";
  const [internalValue, setInternalValue] = useState(
    hasDefaultValue ? defaultValue : "",
  );
  const value = isControlled ? valueFromProps : internalValue;
  const onChange: ChangeEventHandler<HTMLInputElement> = (e) => {
    if (onChangeFromProps) {
      onChangeFromProps(e);
    }
    if (!isControlled) {
      setInternalValue(e.target.value);
    }
  };
  return (
    <>
      <label htmlFor={id} className="ps-1 text-gray-500">
        {label}
      </label>
      <input
        type="text"
        id={id}
        value={value}
        onChange={onChange}
        className={twMerge(
          "mt-1 px-5 py-3 w-full border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
          className,
        )}
        {...props}
      />
    </>
  );
};
