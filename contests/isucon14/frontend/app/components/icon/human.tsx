import type { ComponentProps, FC } from "react";
import colors from "tailwindcss/colors";

export const HumanIcon: FC<ComponentProps<"svg">> = (props) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      height="24px"
      width="24px"
      viewBox="0 -960 960 960"
      fill={colors.neutral[500]}
      {...props}
    >
      <path d="M160-80v-240h120v240H160Zm200 0v-476q-50 17-65 62.5T280-400h-80q0-128 75-204t205-76q100 0 150-49.5T680-880h80q0 88-37.5 157.5T600-624v544h-80v-240h-80v240h-80Zm120-640q-33 0-56.5-23.5T400-800q0-33 23.5-56.5T480-880q33 0 56.5 23.5T560-800q0 33-23.5 56.5T480-720Z" />
    </svg>
  );
};

// https://github.com/google/material-design-icons/blob/master/LICENSE
