import { useEffect, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { AppLogo } from "./AppLogo";

interface PosterImageProps {
  src?: string | null;
  alt: string;
  className?: string;
}

const memoryCache = new Map<string, string>();

function isRemoteSrc(src: string) {
  return /^(https?:|data:)/i.test(src);
}

export function PosterImage({ src, alt, className = "" }: PosterImageProps) {
  const [resolved, setResolved] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    if (!src) {
      setResolved(null);
      setFailed(false);
      return;
    }

    let cancelled = false;
    setFailed(false);

    if (!isRemoteSrc(src)) {
      try {
        setResolved(convertFileSrc(src));
      } catch {
        setResolved(src);
      }
      return () => {
        cancelled = true;
      };
    }

    const cached = memoryCache.get(src);
    if (cached) {
      setResolved(cached);
      return;
    }

    invoke<string | null>("fetch_poster", { url: src })
      .then((dataUrl) => {
        if (cancelled) return;
        if (dataUrl) {
          memoryCache.set(src, dataUrl);
          setResolved(dataUrl);
        } else {
          setResolved(src);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setResolved(src);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [src]);

  const displaySrc = resolved ?? src;

  if (!displaySrc || failed) {
    return (
      <div className={`poster-fallback ${className}`.trim()} aria-label={alt}>
        <AppLogo size={48} />
      </div>
    );
  }

  return (
    <img
      src={displaySrc}
      alt={alt}
      className={className}
      loading="lazy"
      decoding="async"
      onError={() => setFailed(true)}
    />
  );
}
