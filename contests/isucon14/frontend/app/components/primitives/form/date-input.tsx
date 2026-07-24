import { ComponentProps, FC, PropsWithoutRef } from "react";
import { twMerge } from "tailwind-merge";

type DateInputProps = PropsWithoutRef<{
  id: string;
  name: string;
  label?: string;
  className?: string;
}> &
  ComponentProps<"input">;

export const DateInput: FC<DateInputProps> = ({
  label,
  id,
  className,
  ...props
}) => {
  return (
    <>
      {label ? (
        <label htmlFor={id} className="ps-1 text-gray-500">
          {label}
        </label>
      ) : null}
      <input
        type="date"
        id={id}
        className={twMerge(
          "mt-1 px-5 py-3 w-full border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500",
          className,
        )}
        {...props}
      />
    </>
  );
};
