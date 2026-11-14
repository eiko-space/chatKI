import { useState, type FormEvent, type ReactNode } from "react";
import { Cloud, KeyRound, Loader2, LogOut, RefreshCw, UserRound } from "lucide-react";
import { PageHeader } from "../components/ui/PageHeader";
import { Card } from "../components/ui/Card";
import { Badge } from "../components/ui/Badge";
import { Button } from "../components/ui/Button";
import { Details } from "../components/ui/Details";
import { useAuth } from "../lib/auth";
import { testConnection } from "../lib/supabase";

type View = "login" | "register" | "confirm" | "forgot" | "reset" | "invite";

function Field(props: { label: string; type?: string; value: string; onChange: (v: string) => void; autoComplete?: string; inputMode?: "numeric" }) {
  return (
    <label className="block">
      <span className="text-xs font-medium text-muted">{props.label}</span>
      <input
        type={props.type ?? "text"}
        value={props.value}
        autoComplete={props.autoComplete}
        inputMode={props.inputMode}
        onChange={(e) => props.onChange(e.target.value)}
        className="mt-1 h-10 w-full rounded-lg border border-line bg-surface-2 px-3 text-sm text-text focus:border-accent"
        required
      />
    </label>
  );
}

function DiscordIcon() {
  return (
    <svg viewBox="0 0 24 24" className="size-4" fill="currentColor" aria-hidden>
      <path d="M20.3 4.4A19.8 19.8 0 0 0 15.4 3l-.6 1.3a18.4 18.4 0 0 0-5.6 0L8.6 3a19.7 19.7 0 0 0-4.9 1.4C.6 9 0 13.6.3 18.1a19.9 19.9 0 0 0 6 3l1.3-2a12.9 12.9 0 0 1-2-1l.5-.4a14.2 14.2 0 0 0 12 0l.5.4c-.6.4-1.3.7-2 1l1.3 2a19.8 19.8 0 0 0 6-3c.5-5.2-.8-9.7-3.6-13.7ZM8.5 15.3c-1.2 0-2.1-1.1-2.1-2.4s.9-2.4 2.1-2.4 2.2 1.1 2.1 2.4c0 1.3-.9 2.4-2.1 2.4Zm7 0c-1.2 0-2.1-1.1-2.1-2.4s.9-2.4 2.1-2.4 2.2 1.1 2.1 2.4c0 1.3-.9 2.4-2.1 2.4Z" />
    </svg>
  );
}

function GoogleIcon() {
  return (
    <svg viewBox="0 0 24 24" className="size-4" aria-hidden>
      <path fill="#4285F4" d="M22.6 12.2c0-.8-.1-1.5-.2-2.2H12v4.2h6a5 5 0 0 1-2.2 3.3v2.7h3.5c2.1-1.9 3.3-4.7 3.3-8Z" />
      <path fill="#34A853" d="M12 23c3 0 5.5-1 7.3-2.7l-3.5-2.7c-1 .7-2.3 1-3.8 1-2.9 0-5.4-2-6.3-4.6H2v2.8A11 11 0 0 0 12 23Z" />
      <path fill="#FBBC05" d="M5.7 14c-.2-.7-.4-1.3-.4-2s.1-1.4.4-2V7.2H2a11 11 0 0 0 0 9.8L5.7 14Z" />
      <path fill="#EA4335" d="M12 5.4c1.6 0 3.1.6 4.3 1.7l3.1-3.1A11 11 0 0 0 2 7.2L5.7 10c.9-2.7 3.4-4.6 6.3-4.6Z" />
    </svg>
  );
}

export default function Account({ gate = false }: { gate?: boolean }) {
  const auth = useAuth();
  const [view, setView] = useState<View>("login");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const [diag, setDiag] = useState<{ url: string; ok: boolean; detail: string } | null>(null);
  const runDiag = async () => {
    setDiag({ url: "…", ok: false, detail: "Prüfe …" });
    setDiag(await testConnection());
  };

  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setMsg(null);
    try {
      await fn();
    } catch (e) {
      setMsg({ ok: false, text: e instanceof Error ? e.message : String(e) });
    } finally {
      setBusy(false);
    }
  };

  const submit = (fn: () => Promise<void>) => (e: FormEvent) => {
    e.preventDefault();
    run(fn);
  };

  const guestCard = (
    <Card title="Gast-Modus" action={!auth.session ? <Badge tone="accent">Aktiv</Badge> : undefined}>
      <div className="flex items-center gap-3">
        <div className="grid size-11 place-items-center rounded-full bg-surface-3 text-muted"><UserRound size={20} /></div>
        <p className="text-sm text-muted">Alle Funktionen laufen auch ohne Konto. Deine Daten bleiben dann nur auf diesem PC.</p>
      </div>
    </Card>
  );

  if (!auth.configured) {
    return (
      <>
        <PageHeader title="Konto" description="Ein Konto ist optional. Alle Grundfunktionen laufen offline im Gast-Modus." />
        <div className="grid gap-4 md:grid-cols-2">
          {guestCard}
          <Card title="Konto-Funktion noch nicht eingerichtet">
            <p className="text-sm text-muted">
              Für Login und Cloud-Sync braucht GameBoost ein (kostenloses) Supabase-Projekt. Die Anleitung steht in der README unter
              „Konto &amp; Cloud-Sync einrichten“.
            </p>
          </Card>
        </div>
      </>
    );
  }

  if (!auth.ready) {
    return <div className="flex items-center gap-2 text-sm text-muted"><Loader2 size={14} className="animate-spin" /> Lade Konto …</div>;
  }

  if (auth.session) {
    const u = auth.session.user;
    const provider = (u.app_metadata?.provider as string | undefined) ?? "email";
    return (
      <>
        <PageHeader title="Konto" />
        <div className="grid gap-4 md:grid-cols-2">
          <Card title="Angemeldet" action={<Badge tone={auth.plan === "pro" ? "accent" : "neutral"}>{auth.plan === "pro" ? "Pro" : "Free"}</Badge>}>
            <div className="flex items-center gap-3">
              <div className="grid size-11 place-items-center rounded-full bg-accent-dim text-accent"><UserRound size={20} /></div>
              <div className="min-w-0">
                <div className="truncate text-sm font-medium">{u.email}</div>
                <div className="text-xs text-muted">über {provider === "email" ? "E-Mail" : provider === "discord" ? "Discord" : provider === "google" ? "Google" : provider}</div>
              </div>
            </div>
            <div className="mt-5 flex flex-wrap gap-2">
              <Button variant="ghost" onClick={() => run(auth.signOut)} disabled={busy}><LogOut size={14} /> Abmelden</Button>
              {provider === "email" && <Button variant="ghost" onClick={() => setView("reset")}><KeyRound size={14} /> Passwort ändern</Button>}
            </div>
            {view === "reset" && (
              <form className="mt-4 space-y-3" onSubmit={submit(async () => { await auth.updatePassword(password); setMsg({ ok: true, text: "Passwort geändert." }); setView("login"); setPassword(""); })}>
                <Field label="Neues Passwort (mind. 8 Zeichen)" type="password" value={password} onChange={setPassword} autoComplete="new-password" />
                <Button type="submit" variant="primary" disabled={busy}>Speichern</Button>
              </form>
            )}
            {msg && <p className={`mt-3 text-sm ${msg.ok ? "text-good" : "text-bad"}`}>{msg.text}</p>}
          </Card>
          <Card title="Cloud-Sync" action={<Cloud size={16} className="text-faint" />}>
            <p className="text-sm text-muted">
              Einstellungen, Spielprofile und Messungen werden automatisch mit deinem Konto abgeglichen – so hast du sie auch auf anderen PCs.
            </p>
            <p className="mt-3 text-xs text-faint">
              {auth.syncing ? "Synchronisiere …" : auth.lastSync ? `Zuletzt synchronisiert: ${auth.lastSync.toLocaleTimeString("de-DE")}` : "Noch nicht synchronisiert."}
            </p>
            {auth.syncError && <p className="mt-2 text-xs text-bad">{auth.syncError}</p>}
            <Button className="mt-4" onClick={auth.syncNow} disabled={auth.syncing}>
              <RefreshCw size={14} className={auth.syncing ? "animate-spin" : ""} /> Jetzt synchronisieren
            </Button>
            <Details label="Was wird synchronisiert?">
              Tweak-Auswahl, Hintergrund-Apps, Spielprofile, Overlay- und Benachrichtigungs-Einstellungen sowie deine Vorher-/Nachher-Messungen.
              Nicht synchronisiert werden PC-spezifische Dinge wie Programmpfade oder der Wiederherstellungspunkt.
            </Details>
          </Card>
        </div>
      </>
    );
  }

  const Tabs = (
    <div className={`mb-5 grid rounded-lg bg-surface-2 p-1 text-sm ${auth.inviteOnly ? "grid-cols-1" : "grid-cols-2"}`}>
      {(auth.inviteOnly ? (["login"] as const) : (["login", "register"] as const)).map((v) => (
        <button
          key={v}
          onClick={() => { setView(v); setMsg(null); }}
          className={`h-8 rounded-md font-medium transition-colors duration-150 ${view === v ? "bg-surface-3 text-text" : "text-muted hover:text-text"}`}
        >
          {v === "login" ? "Anmelden" : "Registrieren"}
        </button>
      ))}
    </div>
  );

  let form: ReactNode;
  switch (view) {
    case "login":
      form = (
        <form className="space-y-3" onSubmit={submit(() => auth.signIn(email, password))}>
          <Field label="E-Mail" type="email" value={email} onChange={setEmail} autoComplete="email" />
          <Field label="Passwort" type="password" value={password} onChange={setPassword} autoComplete="current-password" />
          <Button type="submit" variant="primary" className="w-full" disabled={busy}>{busy ? "Bitte warten …" : "Anmelden"}</Button>
          <div className="flex flex-wrap justify-between gap-2">
            <button type="button" className="text-xs text-muted hover:text-text" onClick={() => { setView("forgot"); setMsg(null); }}>Passwort vergessen?</button>
            {auth.inviteOnly && (
              <button type="button" className="text-xs text-accent hover:underline" onClick={() => { setView("invite"); setMsg(null); }}>Ich habe eine Einladung</button>
            )}
          </div>
        </form>
      );
      break;
    case "register":
      form = (
        <form className="space-y-3" onSubmit={submit(async () => {
          const r = await auth.signUp(email, password);
          if (r === "confirm") {
            setView("confirm");
            setMsg({ ok: true, text: "Wir haben dir eine E-Mail geschickt. Klicke auf den Link oder gib den Code hier ein." });
          }
        })}>
          <Field label="E-Mail" type="email" value={email} onChange={setEmail} autoComplete="email" />
          <Field label="Passwort (mind. 8 Zeichen)" type="password" value={password} onChange={setPassword} autoComplete="new-password" />
          <Button type="submit" variant="primary" className="w-full" disabled={busy || password.length < 8}>{busy ? "Bitte warten …" : "Konto erstellen"}</Button>
        </form>
      );
      break;
    case "confirm":
      form = (
        <form className="space-y-3" onSubmit={submit(() => auth.verifyCode(email, code, "signup"))}>
          <p className="text-sm text-muted">Bestätigungscode aus der E-Mail an <b className="text-text">{email}</b>:</p>
          <Field label="Code" value={code} onChange={setCode} inputMode="numeric" autoComplete="one-time-code" />
          <Button type="submit" variant="primary" className="w-full" disabled={busy}>Bestätigen</Button>
          <button type="button" className="text-xs text-muted hover:text-text" onClick={() => setView("login")}>Schon per Link bestätigt? Zum Anmelden</button>
        </form>
      );
      break;
    case "forgot":
      form = (
        <form className="space-y-3" onSubmit={submit(async () => {
          await auth.sendReset(email);
          setView("reset");
          setMsg({ ok: true, text: "Falls ein Konto existiert, haben wir dir einen Code geschickt." });
        })}>
          <p className="text-sm text-muted">Gib deine E-Mail ein. Du bekommst einen Code, mit dem du ein neues Passwort setzt.</p>
          <Field label="E-Mail" type="email" value={email} onChange={setEmail} autoComplete="email" />
          <Button type="submit" variant="primary" className="w-full" disabled={busy}>Code anfordern</Button>
          <button type="button" className="text-xs text-muted hover:text-text" onClick={() => setView("login")}>Zurück zum Anmelden</button>
        </form>
      );
      break;
    case "invite":
      form = (
        <form className="space-y-3" onSubmit={submit(async () => {
          await auth.verifyCode(email, code, "invite");
          await auth.updatePassword(password);
        })}>
          <p className="text-sm text-muted">Gib die E-Mail ein, an die deine Einladung ging, den Code aus der E-Mail und dein neues Passwort.</p>
          <Field label="E-Mail" type="email" value={email} onChange={setEmail} autoComplete="email" />
          <Field label="Code aus der Einladung" value={code} onChange={setCode} inputMode="numeric" autoComplete="one-time-code" />
          <Field label="Dein Passwort (mind. 8 Zeichen)" type="password" value={password} onChange={setPassword} autoComplete="new-password" />
          <Button type="submit" variant="primary" className="w-full" disabled={busy || password.length < 8}>Einladung annehmen</Button>
          <button type="button" className="text-xs text-muted hover:text-text" onClick={() => setView("login")}>Zurück zum Anmelden</button>
        </form>
      );
      break;
    case "reset":
      form = (
        <form className="space-y-3" onSubmit={submit(async () => {
          await auth.verifyCode(email, code, "recovery");
          await auth.updatePassword(password);
        })}>
          <Field label="Code aus der E-Mail" value={code} onChange={setCode} inputMode="numeric" autoComplete="one-time-code" />
          <Field label="Neues Passwort (mind. 8 Zeichen)" type="password" value={password} onChange={setPassword} autoComplete="new-password" />
          <Button type="submit" variant="primary" className="w-full" disabled={busy || password.length < 8}>Passwort speichern</Button>
        </form>
      );
      break;
  }

  return (
    <>
      {gate ? (
        <div className="mb-6 text-center">
          <h1 className="text-2xl font-semibold tracking-tight">Willkommen bei GameBoost</h1>
          <p className="mt-1 text-sm text-muted">Diese App ist nur für eingeladene Personen. Bitte melde dich an.</p>
        </div>
      ) : (
        <PageHeader title="Konto" description="Ein Konto ist optional. Damit synchronisierst du Einstellungen, Profile und Messungen über mehrere PCs." />
      )}
      <div className={gate ? "mx-auto max-w-md" : "grid gap-4 md:grid-cols-2"}>
        <Card>
          {(view === "login" || view === "register") && Tabs}
          {form}
          {msg && <p className={`mt-3 text-sm ${msg.ok ? "text-good" : "text-bad"}`}>{msg.text}</p>}
          <div className="mt-3">
            <button type="button" className="text-xs text-muted underline hover:text-text" onClick={runDiag}>Verbindung prüfen</button>
            {diag && (
              <div className="mt-2 rounded-lg bg-surface-2 p-3 text-[11px] leading-relaxed">
                <div className="text-faint">Adresse: <span className="text-text">{diag.url}</span></div>
                <div className={diag.ok ? "text-good" : "text-bad"}>{diag.detail}</div>
              </div>
            )}
          </div>
          {(view === "login" || view === "register") && (
            <>
              {auth.inviteOnly && <p className="mt-4 text-[11px] text-faint">Discord/Google funktionieren nur, wenn für deine E-Mail bereits ein Konto angelegt wurde.</p>}
              <div className="my-5 flex items-center gap-3 text-xs text-faint"><span className="h-px flex-1 bg-line" /> oder <span className="h-px flex-1 bg-line" /></div>
              <div className="grid gap-2 sm:grid-cols-2">
                <Button onClick={() => run(() => auth.signInWithProvider("discord"))} disabled={busy}><DiscordIcon /> Mit Discord</Button>
                <Button onClick={() => run(() => auth.signInWithProvider("google"))} disabled={busy}><GoogleIcon /> Mit Google</Button>
              </div>
              <p className="mt-3 text-[11px] text-faint">Discord/Google öffnen deinen Browser. Danach kommst du automatisch hierher zurück.</p>
            </>
          )}
        </Card>
        {!auth.inviteOnly && guestCard}
      </div>
    </>
  );
}
