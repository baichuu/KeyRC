<script lang="ts">
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, PhysicalPosition } from "@tauri-apps/api/window";
import { onMount } from "svelte";
import { readTextFile, writeTextFile, mkdir } from "@tauri-apps/plugin-fs";
import { homeDir } from "@tauri-apps/api/path";

interface KeyEvent {
	key: string;
	modifiers: string[];
	event_type: string;
}

interface StoredKey {
	key: string;
	shift: boolean;
	ctrl: boolean;
	alt: boolean;
	super: boolean;
}

interface Settings {
	fontSize: number;
	fontFamily: string;
	textColor: string;
}

const defaultSettings: Settings = {
	fontSize: 48,
	fontFamily: "Roboto Mono",
	textColor: "#ffffff",
};

const svgKeys = ["Backspace", "Enter", "Tab", "Space", "CapsLock"];
const WPM_WINDOW_MS = 10000; // 10 second window for WPM calculation

let settings = $state<Settings>({ ...defaultSettings });

function loadSettings() {
	const saved = localStorage.getItem("keyrc-settings");
	if (saved) {
		settings = { ...defaultSettings, ...JSON.parse(saved) };
	}
}

const textAliases: Record<string, string> = {
	Home: "Home",
	End: "End",
	PageUp: "PgUp",
	PageDown: "PgDn", Insert: "Ins", PrintScreen: "PrtSc",
	ScrollLock: "ScrLk",
	Pause: "Pause",
	NumLock: "Num",
  Escape: "Esc",
  Delete: "Del",
};

const shiftedKeys: Record<string, string> = {
	"1": "!", "2": "@", "3": "#", "4": "$", "5": "%",
	"6": "^", "7": "&", "8": "*", "9": "(", "0": ")",
	"-": "_", "=": "+", "[": "{", "]": "}", "\\": "|",
	";": ":", "'": "\"", ",": "<", ".": ">", "/": "?",
	"`": "~",
};

let keyHistory = $state<StoredKey[]>([]);
let activeModifiers = $state<string[]>([]);
let keyTimestamps = $state<number[]>([]);
let wpm = $state(0);
let capsLockOn = $state(false);

function startDrag() {
	getCurrentWindow().startDragging();
}

function isSvgKey(key: string): boolean {
	return svgKeys.includes(key);
}

function isTextAliasKey(key: string): boolean {
	return key in textAliases;
}

function displayKey(stored: StoredKey): string {
	const { key, shift } = stored;
	if (textAliases[key]) return textAliases[key];
	if (key.length === 1) {
		const isUpper = capsLockOn && !shift;
		return isUpper ? key.toUpperCase() : key.toLowerCase();
	}
	return key;
}

function isTypingKey(key: string): boolean {
	return key.length === 1 || key === "Space";
}

function calculateWpm() {
	const now = Date.now();
	keyTimestamps = keyTimestamps.filter(t => now - t < WPM_WINDOW_MS);
	
	if (keyTimestamps.length < 2) {
		wpm = 0;
		return;
	}
	
	const chars = keyTimestamps.length;
	const timeSpanMs = now - keyTimestamps[keyTimestamps.length - 1];
	const minutes = Math.max(timeSpanMs / 60000, WPM_WINDOW_MS / 60000);
	wpm = Math.round((chars / 5) / minutes);
}

function countVisualItems(keys: StoredKey[]): number {
	return keys.reduce((count, k) => {
		let items = 1; // the key itself
		if (k.shift) items++;
		if (k.ctrl) items++;
		if (k.alt) items++;
		if (k.super) items++;
		return count + items;
	}, 0);
}

function getDisplayKeys(): StoredKey[] {
	const result: StoredKey[] = [];
	for (const key of keyHistory) {
		result.push(key);
		if (countVisualItems(result) >= 4) break;
	}
	return result;
}

async function getConfigPath(file: string) {
	const home = await homeDir();
	return `${home.replace(/\/$/, "")}/.config/keyrc/${file}`;
}

async function loadPosition() {
	try {
		const path = await getConfigPath("position");
		const content = await readTextFile(path);
		const [x, y] = content.trim().split("\n").map(Number);
		if (!isNaN(x) && !isNaN(y)) {
			await getCurrentWindow().setPosition(new PhysicalPosition(x, y));
		}
	} catch {
		// No saved position
	}
}

async function savePosition() {
	try {
		const path = await getConfigPath("position");
		const home = await homeDir();
		const dir = `${home.replace(/\/$/, "")}/.config/keyrc`;
		await mkdir(dir, { recursive: true });
		const pos = await getCurrentWindow().outerPosition();
		await writeTextFile(path, `${pos.x}\n${pos.y}`);
	} catch {
		// Ignore write errors
	}
}

onMount(() => {
	loadSettings();
	loadPosition();

	const handleSettingsChange = () => loadSettings();
	window.addEventListener("storage", handleSettingsChange);

	const unlistenPromise = listen<KeyEvent>("key-event", (event) => {
		const { key, modifiers } = event.payload;

		if (key === "CapsLock") {
			capsLockOn = !capsLockOn;
		}

		activeModifiers = modifiers;
		const shift = modifiers.includes("Shift");
		const ctrl = modifiers.includes("Ctrl");
		const alt = modifiers.includes("Alt");
		const superKey = modifiers.includes("Super");
		
		if (isTextAliasKey(key)) {
			keyHistory = [{ key, shift, ctrl, alt, super: superKey }];
		} else {
			const prevWasAlias = keyHistory.length > 0 && isTextAliasKey(keyHistory[0].key);
			if (prevWasAlias) {
				keyHistory = [{ key, shift, ctrl, alt, super: superKey }];
			} else {
				keyHistory = [{ key, shift, ctrl, alt, super: superKey }, ...keyHistory].slice(0, 4);
			}
		}

		if (isTypingKey(key)) {
			keyTimestamps = [Date.now(), ...keyTimestamps].slice(0, 100);
			calculateWpm();
		}
	});

	const unlistenMovePromise = getCurrentWindow().onMoved(() => {
		savePosition();
	});

	const wpmInterval = setInterval(calculateWpm, 1000);
	const settingsInterval = setInterval(loadSettings, 500);

	return () => {
		unlistenPromise.then((unlisten) => unlisten());
		unlistenMovePromise.then((unlisten) => unlisten());
		clearInterval(wpmInterval);
		clearInterval(settingsInterval);
		window.removeEventListener("storage", handleSettingsChange);
	};
});
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<main
	onmousedown={startDrag}
	class="w-full h-screen cursor-move flex flex-col justify-center items-center gap-0.5"
	style="font-family: {settings.fontFamily};"
>
	<div
		class="bg-black/90 h-32 w-full flex flex-row justify-center items-center rounded-tr-3xl rounded-tl-3xl overflow-hidden gap-1 relative"
		style="color: {settings.textColor};"
	>
		{#each getDisplayKeys().toReversed() as stored}
			{#if isSvgKey(stored.key)}
				{#if stored.key === "Backspace"}
<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" style="font-size: {settings.fontSize}px;" viewBox="0 0 24 24"><path fill="currentColor" d="m11.4 16l2.6-2.6l2.6 2.6l1.4-1.4l-2.6-2.6L18 9.4L16.6 8L14 10.6L11.4 8L10 9.4l2.6 2.6l-2.6 2.6zM9 20q-.475 0-.9-.213t-.7-.587L2 12l5.4-7.2q.275-.375.7-.587T9 4h11q.825 0 1.413.587T22 6v12q0 .825-.587 1.413T20 20zm-4.5-8L9 18h11V6H9zm10 0"/></svg>
				{:else if stored.key === "Enter"}
<svg xmlns="http://www.w3.org/2000/svg" style="font-size: {settings.fontSize}px;" width="1em" height="1em" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="square" stroke-width="2" d="M5.75 16H16a3 3 0 0 0 3-3V5M8 12.5L4.5 16L8 19.5"/></svg>
				{:else if stored.key === "Tab"}
<svg xmlns="http://www.w3.org/2000/svg" style="font-size: {settings.fontSize}px;" width="1em" height="1em" viewBox="0 0 16 16"><path fill="currentColor" d="m10.78 8.53l-3.75 3.75a.749.749 0 1 1-1.06-1.06l2.469-2.47H1.75a.75.75 0 0 1 0-1.5h6.689L5.97 4.78a.749.749 0 1 1 1.06-1.06l3.75 3.75a.75.75 0 0 1 0 1.06M13 12.25v-8.5a.75.75 0 0 1 1.5 0v8.5a.75.75 0 0 1-1.5 0"/></svg>
				{:else if stored.key === "Space"}
<svg xmlns="http://www.w3.org/2000/svg" style="font-size: {settings.fontSize}px;" width="1em" height="1em" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 10v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3"/></svg>
				{:else if stored.key === "CapsLock"}
<svg xmlns="http://www.w3.org/2000/svg" style="font-size: {settings.fontSize}px;" width="1em" height="1em" viewBox="0 0 56 56"><path fill="currentColor" d="M20.781 37.621h14.461c3.281 0 5.016-1.922 5.016-5.016v-4.148h8.882c1.946 0 3.493-1.148 3.493-2.953c0-1.102-.563-1.969-1.617-2.883L30.906 4.88c-.96-.844-1.851-1.406-2.906-1.406c-1.031 0-1.922.562-2.883 1.406L4.984 22.645c-1.101.96-1.617 1.757-1.617 2.859c0 1.805 1.547 2.953 3.516 2.953h8.86v4.148c0 3.094 1.757 5.016 5.038 5.016m.375-3.539c-.89 0-1.5-.586-1.5-1.477v-6.89c0-.563-.21-.797-.773-.797H8.664c-.164 0-.234-.07-.234-.187a.33.33 0 0 1 .14-.282L27.508 7.996c.21-.187.328-.258.492-.258s.305.07.492.258L47.453 24.45a.33.33 0 0 1 .14.281c0 .118-.093.188-.257.188H37.14c-.563 0-.774.234-.774.797v6.89c0 .868-.656 1.477-1.5 1.477Zm-1.383 18.445h16.29c2.695 0 4.242-1.5 4.242-4.218v-3.375c0-2.72-1.547-4.266-4.243-4.266H19.773c-2.718 0-4.265 1.57-4.265 4.266v3.375c0 2.695 1.547 4.218 4.265 4.218m.54-3.304c-.82 0-1.266-.422-1.266-1.242v-2.72c0-.82.445-1.288 1.265-1.288h15.211c.797 0 1.242.468 1.242 1.289v2.718c0 .82-.445 1.243-1.242 1.243Z"/></svg>
				{/if}
			{:else}
				{#if stored.super}
					<svg xmlns="http://www.w3.org/2000/svg" style="font-size: {settings.fontSize}px;" width="1em" height="1em" viewBox="0 0 24 24">
						<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15.012 5.977v12.046c0 2.645 3.316 3.954 5.14 2.13c1.825-1.825.516-5.141-2.13-5.141H5.978c-2.645 0-3.953 3.316-2.13 5.14c1.825 1.825 5.142.516 5.142-2.13V5.978c0-2.645-3.317-3.953-5.141-2.13c-1.824 1.825-.516 5.142 2.13 5.142h12.045c2.645 0 3.954-3.317 2.13-5.141s-5.141-.516-5.141 2.13"/>
					</svg>
				{/if}
				{#if stored.ctrl}
					<svg xmlns="http://www.w3.org/2000/svg" style="font-size: {settings.fontSize}px;" width="1em" height="1em" viewBox="0 0 16 16">
						<path fill="currentColor" d="M11.5 7a.5.5 0 0 1-.377-.171l-3.124-3.57l-3.124 3.57a.5.5 0 1 1-.753-.659l3.5-4a.502.502 0 0 1 .752 0l3.5 4a.5.5 0 0 1-.376.83z"/>
					</svg>
				{/if}
				{#if stored.alt}
					<svg xmlns="http://www.w3.org/2000/svg" style="font-size: {settings.fontSize}px;" width="1em" height="1em" viewBox="0 0 24 24">
						<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 5.25h5.625l6.75 13.5H21m-6.75-13.5H21"/>
					</svg>
				{/if}
				{#if stored.shift}
					<svg xmlns="http://www.w3.org/2000/svg" style="font-size: {settings.fontSize}px;" width="1em" height="1em" viewBox="0 0 16 16">
						<path fill="currentColor" d="M7.27 2.047a1 1 0 0 1 1.46 0l6.345 6.77c.6.638.146 1.683-.73 1.683H11.5v3a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-3H1.654C.78 10.5.326 9.455.924 8.816zM14.346 9.5L8 2.731L1.654 9.5H4.5a1 1 0 0 1 1 1v3h5v-3a1 1 0 0 1 1-1z"/>
					</svg>
				{/if}
				<span style="font-size: {settings.fontSize}px;">{displayKey(stored)}</span>
			{/if}
		{/each}
    <p class="absolute top-2 right-3 text-xl opacity-30 text-center flex items-center gap-1">
      <svg xmlns="http://www.w3.org/2000/svg" width="1.6em" height="1.6em" viewBox="0 0 24 24"><title xmlns="">keyboard-outline-rounded</title><path fill="currentColor" d="M4.616 18q-.691 0-1.153-.462T3 16.384V7.616q0-.691.463-1.153T4.615 6h14.77q.69 0 1.152.463T21 7.616v8.769q0 .69-.463 1.153T19.385 18zm0-1h14.769q.23 0 .423-.192t.192-.424V7.616q0-.231-.192-.424T19.385 7H4.615q-.23 0-.423.192T4 7.616v8.769q0 .23.192.423t.423.192M9 15.77h6q.31 0 .54-.221q.23-.22.23-.549q0-.31-.23-.54t-.54-.23H9q-.31 0-.54.221q-.23.22-.23.549q0 .31.23.54t.54.23M4 17V7zm2-7.23q.31 0 .54-.23T6.77 9t-.23-.54T6 8.23t-.54.23t-.23.54t.23.54t.54.23m3 0q.31 0 .54-.23T9.77 9t-.23-.54T9 8.23t-.54.23t-.23.54t.23.54t.54.23m3 0q.31 0 .54-.23t.23-.54t-.23-.54t-.54-.23t-.54.23t-.23.54t.23.54t.54.23m3 0q.31 0 .54-.23t.23-.54t-.23-.54t-.54-.23t-.54.23t-.23.54t.23.54t.54.23m3 0q.31 0 .54-.23t.23-.54t-.23-.54t-.54-.23t-.54.23t-.23.54t.23.54t.54.23m-12 3q.31 0 .54-.23t.23-.54t-.23-.54t-.54-.23t-.54.23t-.23.54t.23.54t.54.23m3 0q.31 0 .54-.23t.23-.54t-.23-.54t-.54-.23t-.54.23t-.23.54t.23.54t.54.23m3 0q.31 0 .54-.23t.23-.54t-.23-.54t-.54-.23t-.54.23t-.23.54t.23.54t.54.23m3 0q.31 0 .54-.23t.23-.54t-.23-.54t-.54-.23t-.54.23t-.23.54t.23.54t.54.23m3 0q.31 0 .54-.23t.23-.54t-.23-.54t-.54-.23t-.54.23t-.23.54t.23.54t.54.23"/></svg> {wpm}
    </p>
	</div>
	<div
		class="h-14 shrink-0 grid grid-cols-4 gap-0.5 w-full rounded-br-3xl rounded-bl-3xl overflow-hidden"
	>
		<div class="flex items-center justify-center bg-black/90 {activeModifiers.includes('Shift') ? 'text-white' : 'text-white/30'}">
			<svg xmlns="http://www.w3.org/2000/svg" width="1.6em" height="1.6em" viewBox="0 0 16 16">
				<path fill="currentColor" d="M7.27 2.047a1 1 0 0 1 1.46 0l6.345 6.77c.6.638.146 1.683-.73 1.683H11.5v3a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-3H1.654C.78 10.5.326 9.455.924 8.816zM14.346 9.5L8 2.731L1.654 9.5H4.5a1 1 0 0 1 1 1v3h5v-3a1 1 0 0 1 1-1z"/>
			</svg>
		</div>
		<div class="flex items-center justify-center bg-black/90 {activeModifiers.includes('Ctrl') ? 'text-white' : 'text-white/30'}">
			<svg xmlns="http://www.w3.org/2000/svg" width="1.8em" height="1.8em" viewBox="0 0 16 16">
				<path fill="currentColor" d="M11.5 7a.5.5 0 0 1-.377-.171l-3.124-3.57l-3.124 3.57a.5.5 0 1 1-.753-.659l3.5-4a.502.502 0 0 1 .752 0l3.5 4a.5.5 0 0 1-.376.83z"/>
			</svg>
		</div>
		<div class="flex items-center justify-center bg-black/90 {activeModifiers.includes('Alt') ? 'text-white' : 'text-white/30'}">
			<svg xmlns="http://www.w3.org/2000/svg" width="1.6em" height="1.6em" viewBox="0 0 24 24">
				<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 5.25h5.625l6.75 13.5H21m-6.75-13.5H21"/>
			</svg>
		</div>
		<div class="flex items-center justify-center bg-black/90 {activeModifiers.includes('Super') ? 'text-white' : 'text-white/30'}">
			<svg xmlns="http://www.w3.org/2000/svg" width="1.6em" height="1.6em" viewBox="0 0 24 24">
				<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15.012 5.977v12.046c0 2.645 3.316 3.954 5.14 2.13c1.825-1.825.516-5.141-2.13-5.141H5.978c-2.645 0-3.953 3.316-2.13 5.14c1.825 1.825 5.142.516 5.142-2.13V5.978c0-2.645-3.317-3.953-5.141-2.13c-1.824 1.825-.516 5.142 2.13 5.142h12.045c2.645 0 3.954-3.317 2.13-5.141s-5.141-.516-5.141 2.13"/>
			</svg>
		</div>
	</div>
</main>
