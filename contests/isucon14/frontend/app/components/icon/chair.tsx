import { ComponentProps, FC, useMemo } from "react";
import colors from "tailwindcss/colors";

const ChairTypes = [
  {
    type: "green",
    bodyColor: "#87c9a1",
    windowColor: "#00a0e9",
    wheelColor: "#e0803a",
  },
  {
    type: "yellow",
    bodyColor: "#ffa455",
    windowColor: "#ffc155",
    wheelColor: "#226b9f",
  },
  { type: "red", bodyColor: "#f63", windowColor: "#f93", wheelColor: "#099" },
  {
    type: "gray",
    bodyColor: colors.neutral[300],
    windowColor: colors.neutral[400],
    wheelColor: colors.neutral[600],
  },
  {
    type: "sky",
    bodyColor: colors.sky[300],
    windowColor: colors.sky[500],
    wheelColor: "#00a0e9",
  },
] as const;

const Chair: FC<
  {
    chairType: (typeof ChairTypes)[number];
  } & ComponentProps<"svg">
> = ({ chairType, ...props }) => {
  const { bodyColor, windowColor, wheelColor } = chairType;
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 395.5 390.5"
      {...props}
    >
      <path
        d="M163.39 63.5h68.21c32.78 0 59.39 26.61 59.39 59.39V398.1c0 32.78-26.61 59.39-59.39 59.39h-68.21c-32.78 0-59.39-26.61-59.39-59.39V122.89c0-32.78 26.61-59.39 59.39-59.39Z"
        transform="rotate(-90 197.5 260.5)"
        strokeMiterlimit="10"
        fill={bodyColor}
        stroke={bodyColor}
      />
      <path
        d="M335.49 354h-68.35c-25.92-20.38-45.56-39-59.51-53.29-13.34-13.66-23.38-25.32-32.06-36.43-14.39-18.42-20.78-29.89-23.05-34.09-5.17-9.59-8.69-18-11.02-24.22 2.69-6.87 64-163.39 76.15-181.24 3.24-4.76 7.41-8.62 7.41-8.62 2.15-1.99 10.48-9.34 24.38-13.21 5.59-1.56 11.53-2.4 17.7-2.4h68.35C368.36.5 395 24.36 395 53.79v246.92c0 29.43-26.64 53.29-59.51 53.29Z"
        strokeMiterlimit="10"
        fill={bodyColor}
        stroke={bodyColor}
      />
      <path
        d="M237.5 16.75v119.52c0 16.97-16.61 30.72-37.09 30.72h-39.93c-1.05 0-1.78-.86-1.42-1.68l46.16-109.65c.19-.45.39-.89.61-1.33l13.5-27.01c.7-1.41 1.55-2.74 2.53-3.96.89-1.1 1.87-2.39 2.86-3.66a22.4 22.4 0 0 1 8.93-6.92l1.86-.8c.98-.68 2 3.69 2 4.75Z"
        strokeMiterlimit="10"
        fill={windowColor}
        stroke={windowColor}
      />
      <ellipse
        cx="198.5"
        cy="304"
        rx="86.25"
        ry="86"
        strokeMiterlimit="10"
        fill={wheelColor}
        stroke={wheelColor}
      />
    </svg>
  );
};

export const ChairIcon: FC<{ model: string } & ComponentProps<"svg">> = ({
  model,
  ...props
}) => {
  const chairType = useMemo(() => {
    return ChairTypes[
      model ? (model.charCodeAt(0) + model.length) % ChairTypes.length : 0
    ];
  }, [model]);
  return <Chair chairType={chairType} {...props} />;
};

export const ChairTypeIcon: FC<
  { type: (typeof ChairTypes)[number]["type"] } & ComponentProps<"svg">
> = ({ type, ...props }) => {
  const chairType = useMemo(
    () => ChairTypes.find((c) => c.type === type) ?? ChairTypes[0],
    [type],
  );
  return <Chair chairType={chairType} {...props} />;
};
