import type { ComponentProps, FC } from "react";
import { twMerge } from "tailwind-merge";
import { Coordinate } from "~/api/api-schemas";
import { PinIcon } from "~/components/icon/pin";
import { Button } from "~/components/primitives/button/button";

type Direction = "from" | "to";

type LocationButtonProps = {
  direction?: Direction;
  location?: Coordinate;
  label?: string;
  disabled?: boolean;
  className?: string;
  placeholder?: string;
  onClick?: () => void;
} & ComponentProps<typeof Button>;

export const LocationButton: FC<LocationButtonProps> = ({
  direction,
  location,
  label,
  disabled,
  className,
  onClick,
  placeholder = "場所を選択する",
  ...props
}) => {
  return (
    <Button
      disabled={disabled}
      className={twMerge("relative", className)}
      onClick={onClick}
      {...props}
    >
      {direction === "to" && <PinIcon />}
      {label && (
        <span className="absolute flex items-center h-full top-0 left-4 text-xs text-neutral-500 font-mono">
          {label}
        </span>
      )}
      {location ? (
        <span className="font-mono">
          {`[${location.latitude}, ${location.longitude}]`}
        </span>
      ) : (
        <span>{placeholder}</span>
      )}
    </Button>
  );
};
