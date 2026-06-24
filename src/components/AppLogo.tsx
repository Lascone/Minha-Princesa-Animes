interface AppLogoProps {
  className?: string;
  size?: number;
}

export function AppLogo({ className = "", size = 52 }: AppLogoProps) {
  return (
    <img
      src="/icon.png"
      alt="Minha Princesa"
      className={`app-logo ${className}`.trim()}
      width={size}
      height={size}
      draggable={false}
    />
  );
}
