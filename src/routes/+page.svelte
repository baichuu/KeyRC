<script lang="ts">
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { onMount } from "svelte";

interface KeyEvent {
	key: string;
	modifiers: string[];
	event_type: string;
}

interface StoredKey {
	key: string;
	shift: boolean;
}

const svgKeys = ["Backspace", "Enter", "Tab", "Space", "CapsLock"];
const WPM_WINDOW_MS = 10000; // 10 second window for WPM calculation

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
	if (shift && shiftedKeys[key]) return shiftedKeys[key];
	if (key.length === 1) {
		return shift ? key.toUpperCase() : key.toLowerCase();
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

function getDisplayKeys(): StoredKey[] {
	return keyHistory;
}

onMount(() => {
	const unlistenPromise = listen<KeyEvent>("key-event", (event) => {
		const { key, modifiers } = event.payload;

		activeModifiers = modifiers;
		const shift = modifiers.includes("Shift");
		
		if (isTextAliasKey(key)) {
			keyHistory = [{ key, shift }];
		} else {
			const prevWasAlias = keyHistory.length > 0 && isTextAliasKey(keyHistory[0].key);
			if (prevWasAlias) {
				keyHistory = [{ key, shift }];
			} else {
				keyHistory = [{ key, shift }, ...keyHistory].slice(0, 4);
			}
		}

		if (isTypingKey(key)) {
			keyTimestamps = [Date.now(), ...keyTimestamps].slice(0, 100);
			calculateWpm();
		}
	});

	const wpmInterval = setInterval(calculateWpm, 1000);

	return () => {
		unlistenPromise.then((unlisten) => unlisten());
		clearInterval(wpmInterval);
	};
});
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<main
	onmousedown={startDrag}
	class="w-full h-screen cursor-move flex flex-col justify-center items-center gap-0.5"
>
	<div
		class="bg-black/90 h-32 w-full flex flex-row justify-center items-center rounded-tr-3xl rounded-tl-3xl overflow-hidden gap-1 relative"
	>
		{#each getDisplayKeys().toReversed() as stored}
			{#if isSvgKey(stored.key)}
				{#if stored.key === "Backspace"}
<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" class="text-5xl" viewBox="0 0 24 24"><title xmlns="">backspace-outline</title><path fill="currentColor" d="m11.4 16l2.6-2.6l2.6 2.6l1.4-1.4l-2.6-2.6L18 9.4L16.6 8L14 10.6L11.4 8L10 9.4l2.6 2.6l-2.6 2.6zM9 20q-.475 0-.9-.213t-.7-.587L2 12l5.4-7.2q.275-.375.7-.587T9 4h11q.825 0 1.413.587T22 6v12q0 .825-.587 1.413T20 20zm-4.5-8L9 18h11V6H9zm10 0"/></svg>
				{:else if stored.key === "Enter"}
<svg xmlns="http://www.w3.org/2000/svg" class="text-5xl" width="1em" height="1em" viewBox="0 0 24 24"><title xmlns="">enter</title><path fill="none" stroke="currentColor" stroke-linecap="square" stroke-width="2" d="M5.75 16H16a3 3 0 0 0 3-3V5M8 12.5L4.5 16L8 19.5"/></svg>
				{:else if stored.key === "Tab"}
<svg xmlns="http://www.w3.org/2000/svg" class="text-5xl" width="1em" height="1em" viewBox="0 0 16 16"><title xmlns="">tab-16</title><path fill="currentColor" d="m10.78 8.53l-3.75 3.75a.749.749 0 1 1-1.06-1.06l2.469-2.47H1.75a.75.75 0 0 1 0-1.5h6.689L5.97 4.78a.749.749 0 1 1 1.06-1.06l3.75 3.75a.75.75 0 0 1 0 1.06M13 12.25v-8.5a.75.75 0 0 1 1.5 0v8.5a.75.75 0 0 1-1.5 0"/></svg>
				{:else if stored.key === "Space"}
<svg xmlns="http://www.w3.org/2000/svg" class="text-5xl" width="1em" height="1em" viewBox="0 0 24 24"><title xmlns="">space</title><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 10v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3"/></svg>
				{/if}
        {:else if stored.key === "CapsLock"}
        <svg xmlns="http://www.w3.org/2000/svg" class="text-5xl" width="1em" height="1em" viewBox="0 0 56 56"><title xmlns="">capslock</title><path fill="currentColor" d="M20.781 37.621h14.461c3.281 0 5.016-1.922 5.016-5.016v-4.148h8.882c1.946 0 3.493-1.148 3.493-2.953c0-1.102-.563-1.969-1.617-2.883L30.906 4.88c-.96-.844-1.851-1.406-2.906-1.406c-1.031 0-1.922.562-2.883 1.406L4.984 22.645c-1.101.96-1.617 1.757-1.617 2.859c0 1.805 1.547 2.953 3.516 2.953h8.86v4.148c0 3.094 1.757 5.016 5.038 5.016m.375-3.539c-.89 0-1.5-.586-1.5-1.477v-6.89c0-.563-.21-.797-.773-.797H8.664c-.164 0-.234-.07-.234-.187a.33.33 0 0 1 .14-.282L27.508 7.996c.21-.187.328-.258.492-.258s.305.07.492.258L47.453 24.45a.33.33 0 0 1 .14.281c0 .118-.093.188-.257.188H37.14c-.563 0-.774.234-.774.797v6.89c0 .868-.656 1.477-1.5 1.477Zm-1.383 18.445h16.29c2.695 0 4.242-1.5 4.242-4.218v-3.375c0-2.72-1.547-4.266-4.243-4.266H19.773c-2.718 0-4.265 1.57-4.265 4.266v3.375c0 2.695 1.547 4.218 4.265 4.218m.54-3.304c-.82 0-1.266-.422-1.266-1.242v-2.72c0-.82.445-1.288 1.265-1.288h15.211c.797 0 1.242.468 1.242 1.289v2.718c0 .82-.445 1.243-1.242 1.243Z"/></svg>
			{:else}
				<span class="text-5xl">{displayKey(stored)}</span>
			{/if}
		{/each}
    <p class="absolute top-2 right-3 text-xl text-white/30 text-center">
      {wpm}
    </p>
	</div>
	<div
		class="h-14 shrink-0 grid grid-cols-4 gap-0.5 w-full rounded-br-3xl rounded-bl-3xl overflow-hidden"
	>
		<div class="flex items-center justify-center bg-black/90 {activeModifiers.includes('Shift') ? 'text-white' : 'text-white/30'}">
			<svg xmlns="http://www.w3.org/2000/svg" width="1.6em" height="1.6em" viewBox="0 0 16 16">
				<title>shift</title>
				<path fill="currentColor" d="M7.27 2.047a1 1 0 0 1 1.46 0l6.345 6.77c.6.638.146 1.683-.73 1.683H11.5v3a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-3H1.654C.78 10.5.326 9.455.924 8.816zM14.346 9.5L8 2.731L1.654 9.5H4.5a1 1 0 0 1 1 1v3h5v-3a1 1 0 0 1 1-1z"/>
			</svg>
		</div>
		<div class="flex items-center justify-center bg-black/90 {activeModifiers.includes('Ctrl') ? 'text-white' : 'text-white/30'}">
			<svg xmlns="http://www.w3.org/2000/svg" width="1.8em" height="1.8em" viewBox="0 0 16 16">
				<title>ctrl</title>
				<path fill="currentColor" d="M11.5 7a.5.5 0 0 1-.377-.171l-3.124-3.57l-3.124 3.57a.5.5 0 1 1-.753-.659l3.5-4a.502.502 0 0 1 .752 0l3.5 4a.5.5 0 0 1-.376.83z"/>
			</svg>
		</div>
		<div class="flex items-center justify-center bg-black/90 {activeModifiers.includes('Alt') ? 'text-white' : 'text-white/30'}">
			<svg xmlns="http://www.w3.org/2000/svg" width="1.6em" height="1.6em" viewBox="0 0 24 24">
				<title>alt</title>
				<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 5.25h5.625l6.75 13.5H21m-6.75-13.5H21"/>
			</svg>
		</div>
		<div class="flex items-center justify-center bg-black/90 {activeModifiers.includes('Super') ? 'text-white' : 'text-white/30'}">
			<svg xmlns="http://www.w3.org/2000/svg" width="1.6em" height="1.6em" viewBox="0 0 24 24">
				<title>command</title>
				<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15.012 5.977v12.046c0 2.645 3.316 3.954 5.14 2.13c1.825-1.825.516-5.141-2.13-5.141H5.978c-2.645 0-3.953 3.316-2.13 5.14c1.825 1.825 5.142.516 5.142-2.13V5.978c0-2.645-3.317-3.953-5.141-2.13c-1.824 1.825-.516 5.142 2.13 5.142h12.045c2.645 0 3.954-3.317 2.13-5.141s-5.141-.516-5.141 2.13"/>
			</svg>
		</div>
	</div>
</main>
