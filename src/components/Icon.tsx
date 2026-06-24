interface IconProps {
  name: string;
  className?: string;
  spin?: boolean;
}

export function Icon({ name, className = "", spin }: IconProps) {
  return (
    <i
      className={`fa-solid ${name}${spin ? " fa-spin" : ""} ${className}`.trim()}
      aria-hidden
    />
  );
}
