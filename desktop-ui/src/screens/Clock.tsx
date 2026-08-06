import { useEffect, useState } from "react";

/**
 * Ambient clock. Shown as a screen, and taken over the whole app after the
 * configured inactivity timeout (see App). `idle` styles it as a full takeover.
 */
export default function Clock({ idle = false }: { idle?: boolean }) {
  const [now, setNow] = useState(new Date());

  useEffect(() => {
    const t = setInterval(() => setNow(new Date()), 1000);
    return () => clearInterval(t);
  }, []);

  const hh = now.getHours();
  const mm = now.getMinutes();
  const ss = now.getSeconds();
  const pad = (n: number) => n.toString().padStart(2, "0");
  const date = now.toLocaleDateString(undefined, {
    weekday: "long",
    month: "long",
    day: "numeric",
  });

  return (
    <div className={`clock ${idle ? "clock-idle" : ""}`}>
      <div className="clock-time">
        <span>{pad(hh)}</span>
        <span className="clock-colon">:</span>
        <span>{pad(mm)}</span>
        <span className="clock-sec">{pad(ss)}</span>
      </div>
      <div className="clock-date">{date}</div>
      {idle && <div className="clock-hint">move the mouse or press a key to resume</div>}
    </div>
  );
}
