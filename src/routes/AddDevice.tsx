import KeyRound from "lucide-solid/icons/key-round";
import LoaderCircle from "lucide-solid/icons/loader-circle";
import Radar from "lucide-solid/icons/radar";
import TextCursorInput from "lucide-solid/icons/text-cursor-input";
import { For, type JSX, Show, createRoot, createSignal } from "solid-js";
import { Button } from "../components/Button";
import { Pill } from "../components/Pill";
import { t } from "../i18n";
import {
  type ConnectStatus,
  type Discovered,
  type DiscoveryKind,
  connect,
  describeError,
  isAdbError,
  pair,
  scan,
} from "../lib/adb";
import { devices } from "../lib/devices";
import styles from "./AddDevice.module.css";
import page from "./Page.module.css";

const IDS = {
  ip: "add-device-ip",
  ipMessage: "add-device-ip-message",
  pairAddress: "add-device-pair-address",
  pairCode: "add-device-pair-code",
  pairMessage: "add-device-pair-message",
  scanMessage: "add-device-scan-message",
} as const;

type Tone = "error" | "warning" | "success";

interface Feedback {
  tone: Tone;
  text: string;
}

const KIND_LABEL: Record<DiscoveryKind, () => string> = {
  wireless: () => t("add.kindWireless"),
  pairing: () => t("add.kindPairing"),
  network: () => t("add.kindNetwork"),
};

function connectFeedback(status: ConnectStatus, address: string): Feedback {
  return status === "connected"
    ? { tone: "success", text: t("add.connected", { address }) }
    : { tone: "warning", text: t("add.unauthorized") };
}

function isAdded(address: string): boolean {
  return devices.some((device) => device.address === address);
}

/**
 * Screen state kept outside the components, so leaving "Add a TV" and coming back keeps scan
 * results, typed addresses and messages; a scan or connect still in flight finishes in the background.
 */
const saved = createRoot(() => {
  const [pairAddress, setPairAddress] = createSignal("");
  const [scanning, setScanning] = createSignal(false);
  const [results, setResults] = createSignal<Discovered[] | null>(null);
  const [connecting, setConnecting] = createSignal<string | null>(null);
  const [scanFeedback, setScanFeedback] = createSignal<Feedback | null>(null);
  const [pairPending, setPairPending] = createSignal(false);
  const [pairFeedback, setPairFeedback] = createSignal<Feedback | null>(null);
  const [manualAddress, setManualAddress] = createSignal("");
  const [manualPending, setManualPending] = createSignal(false);
  const [manualFeedback, setManualFeedback] = createSignal<Feedback | null>(null);
  return {
    pairAddress,
    setPairAddress,
    scanning,
    setScanning,
    results,
    setResults,
    connecting,
    setConnecting,
    scanFeedback,
    setScanFeedback,
    pairPending,
    setPairPending,
    pairFeedback,
    setPairFeedback,
    manualAddress,
    setManualAddress,
    manualPending,
    setManualPending,
    manualFeedback,
    setManualFeedback,
  };
});

export function AddDevice() {
  const { pairAddress, setPairAddress } = saved;
  let pairCodeInput: HTMLInputElement | undefined;

  const usePairing = (address: string) => {
    setPairAddress(address);
    pairCodeInput?.focus();
  };

  return (
    <div class={page.page}>
      <header class={page.header}>
        <h1 class={page.title}>{t("add.title")}</h1>
        <p class={page.lead}>{t("add.lead")}</p>
      </header>

      <ul class={styles.methods}>
        <Method icon={<Radar />} title={t("add.scanTitle")} body={t("add.scanBody")}>
          <ScanSection onUsePairing={usePairing} />
        </Method>
        <Method icon={<KeyRound />} title={t("add.pairTitle")} body={t("add.pairBody")}>
          <PairForm
            address={pairAddress()}
            onAddressInput={setPairAddress}
            codeRef={(element) => {
              pairCodeInput = element;
            }}
          />
        </Method>
        <Method icon={<TextCursorInput />} title={t("add.manualTitle")} body={t("add.manualBody")}>
          <ManualForm />
        </Method>
      </ul>
    </div>
  );
}

function ScanSection(props: { onUsePairing: (address: string) => void }) {
  const { scanning, setScanning, results, setResults, connecting, setConnecting } = saved;
  const feedback = saved.scanFeedback;
  const setFeedback = saved.setScanFeedback;

  const runScan = async () => {
    setScanning(true);
    setFeedback(null);
    try {
      setResults(await scan());
    } catch (error) {
      setResults(null);
      setFeedback({ tone: "error", text: describeError(error) });
    } finally {
      setScanning(false);
    }
  };

  const connectTo = async (address: string) => {
    setConnecting(address);
    setFeedback(null);
    try {
      setFeedback(connectFeedback(await connect(address), address));
    } catch (error) {
      setFeedback({ tone: "error", text: describeError(error) });
    } finally {
      setConnecting(null);
    }
  };

  return (
    <div class={styles.methodBodyStack}>
      <div>
        <Button variant={results() ? "secondary" : "primary"} onClick={runScan} disabled={scanning()}>
          <Show when={scanning()} fallback={<Radar />}>
            <LoaderCircle class={styles.spin} />
          </Show>
          {scanning() ? t("add.scanning") : results() ? t("add.scanAgain") : t("add.scan")}
        </Button>
      </div>

      <Show when={results()}>
        {(found) => (
          <Show when={found().length > 0} fallback={<p class={styles.hint}>{t("add.scanEmpty")}</p>}>
            <div class={styles.results} aria-busy={scanning()}>
              <h3 class={page.sectionTitle}>{t("add.scanResults")}</h3>
              <ul class={styles.resultList}>
                <For each={found()}>
                  {(item) => (
                    <li class={styles.result}>
                      <span class={styles.resultText}>
                        <span class={`${styles.resultAddress} mono`}>{item.address}</span>
                        <span class={styles.resultKind}>{KIND_LABEL[item.kind]()}</span>
                      </span>
                      <Show
                        when={item.kind !== "pairing"}
                        fallback={
                          <Button variant="ghost" onClick={() => props.onUsePairing(item.address)}>
                            {t("add.usePairing")}
                          </Button>
                        }
                      >
                        <Show when={!isAdded(item.address)} fallback={<Pill tone="accent">{t("add.added")}</Pill>}>
                          <Button
                            onClick={() => connectTo(item.address)}
                            disabled={connecting() !== null}
                            aria-describedby={IDS.scanMessage}
                          >
                            <Show when={connecting() === item.address}>
                              <LoaderCircle class={styles.spin} />
                            </Show>
                            {connecting() === item.address ? t("add.connecting") : t("add.connect")}
                          </Button>
                        </Show>
                      </Show>
                    </li>
                  )}
                </For>
              </ul>
            </div>
          </Show>
        )}
      </Show>

      <Message id={IDS.scanMessage} feedback={feedback()} />
    </div>
  );
}

interface PairFormProps {
  address: string;
  onAddressInput: (value: string) => void;
  codeRef: (element: HTMLInputElement) => void;
}

function PairForm(props: PairFormProps) {
  // The pairing code is single-use, so it is not kept when leaving the screen.
  const [code, setCode] = createSignal("");
  const pending = saved.pairPending;
  const setPending = saved.setPairPending;
  const feedback = saved.pairFeedback;
  const setFeedback = saved.setPairFeedback;
  const [invalid, setInvalid] = createSignal<"address" | "code" | null>(null);

  const submit = async (event: SubmitEvent) => {
    event.preventDefault();
    setPending(true);
    setFeedback(null);
    setInvalid(null);
    try {
      const result = await pair(props.address, code());
      setCode("");
      setFeedback(
        result.connection
          ? connectFeedback(result.connection.status, result.connection.address)
          : { tone: "success", text: t("add.paired") },
      );
    } catch (error) {
      if (isAdbError(error) && error.kind === "invalidAddress") setInvalid("address");
      if (isAdbError(error) && (error.kind === "invalidCode" || error.kind === "pairFailed")) setInvalid("code");
      setFeedback({ tone: "error", text: describeError(error) });
    } finally {
      setPending(false);
    }
  };

  return (
    <form class={styles.pairForm} onSubmit={submit} novalidate>
      <div class={styles.field}>
        <label class={styles.label} for={IDS.pairAddress}>
          {t("add.pairAddressLabel")}
        </label>
        <input
          id={IDS.pairAddress}
          class={`${styles.input} mono`}
          inputmode="decimal"
          autocomplete="off"
          spellcheck={false}
          placeholder={t("add.pairAddressPlaceholder")}
          value={props.address}
          onInput={(event) => props.onAddressInput(event.currentTarget.value)}
          aria-invalid={invalid() === "address"}
          aria-describedby={IDS.pairMessage}
          disabled={pending()}
        />
      </div>
      <div class={styles.field}>
        <label class={styles.label} for={IDS.pairCode}>
          {t("add.pairCodeLabel")}
        </label>
        <input
          ref={props.codeRef}
          id={IDS.pairCode}
          class={`${styles.input} ${styles.codeInput} mono`}
          inputmode="numeric"
          autocomplete="one-time-code"
          maxlength={6}
          spellcheck={false}
          placeholder={t("add.pairCodePlaceholder")}
          value={code()}
          onInput={(event) => setCode(event.currentTarget.value)}
          aria-invalid={invalid() === "code"}
          aria-describedby={IDS.pairMessage}
          disabled={pending()}
        />
      </div>
      <Button type="submit" variant="primary" disabled={pending()}>
        <Show when={pending()}>
          <LoaderCircle class={styles.spin} />
        </Show>
        {pending() ? t("add.pairing") : t("add.pair")}
      </Button>
      <Message id={IDS.pairMessage} feedback={feedback()} class={styles.formMessage} />
    </form>
  );
}

function ManualForm() {
  const address = saved.manualAddress;
  const setAddress = saved.setManualAddress;
  const pending = saved.manualPending;
  const setPending = saved.setManualPending;
  const feedback = saved.manualFeedback;
  const setFeedback = saved.setManualFeedback;

  const submit = async (event: SubmitEvent) => {
    event.preventDefault();
    const value = address().trim();
    setPending(true);
    setFeedback(null);
    try {
      setFeedback(connectFeedback(await connect(value), value));
    } catch (error) {
      setFeedback({ tone: "error", text: describeError(error) });
    } finally {
      setPending(false);
    }
  };

  return (
    <form class={styles.ipForm} onSubmit={submit} novalidate>
      <div class={styles.ipRow}>
        <label class="visually-hidden" for={IDS.ip}>
          {t("add.ipLabel")}
        </label>
        <input
          id={IDS.ip}
          class={`${styles.input} mono`}
          inputmode="decimal"
          autocomplete="off"
          spellcheck={false}
          placeholder={t("add.ipPlaceholder")}
          value={address()}
          onInput={(event) => setAddress(event.currentTarget.value)}
          aria-invalid={feedback()?.tone === "error"}
          aria-describedby={IDS.ipMessage}
          disabled={pending()}
        />
        <Button type="submit" variant="primary" disabled={pending() || address().trim() === ""}>
          <Show when={pending()}>
            <LoaderCircle class={styles.spin} />
          </Show>
          {pending() ? t("add.connecting") : t("add.connect")}
        </Button>
      </div>
      <Message id={IDS.ipMessage} feedback={feedback()} />
    </form>
  );
}

/** Result line under a form. Errors interrupt (alert); success and warnings are polite (status). */
function Message(props: { id: string; feedback: Feedback | null; class?: string }) {
  return (
    <p
      id={props.id}
      class={`${styles.message} ${props.feedback ? styles[props.feedback.tone] : ""} ${props.class ?? ""}`}
      role={props.feedback?.tone === "error" ? "alert" : "status"}
    >
      {props.feedback?.text}
    </p>
  );
}

interface MethodProps {
  icon: JSX.Element;
  title: string;
  body: string;
  children?: JSX.Element;
}

function Method(props: MethodProps) {
  return (
    <li class={styles.method}>
      <span class={styles.methodIcon} aria-hidden="true">
        {props.icon}
      </span>
      <div class={styles.methodText}>
        <h2 class={styles.methodTitle}>{props.title}</h2>
        <p class={styles.methodBody}>{props.body}</p>
        {props.children}
      </div>
    </li>
  );
}
