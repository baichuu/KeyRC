<script lang="ts">
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { onMount } from "svelte";

interface KeyEvent {
	key: string;
	modifiers: ModifierState;
}

interface ModifierState {
	shift: boolean;
	ctrl: boolean;
	alt: boolean;
	super_key: boolean;
}

interface StoredKey {
	id: number;
	key: string;
	shift: boolean;
	ctrl: boolean;
	alt: boolean;
	super: boolean;
}

interface Theme {
	mode: "full" | "keys_only";
	active_bg: string;
	active_fg: string;
	key_text: string;
	background: string;
	border: string;
}

interface ThemeWindow extends Window {
	__KEYRC_INITIAL_THEME__?: Theme;
}

const initialTheme = typeof window !== "undefined"
	? (window as ThemeWindow).__KEYRC_INITIAL_THEME__
	: undefined;

const FONT_SIZE = 36;
const ARROW_FONT_SIZE = 44;
const MAX_DISPLAY_UNITS = 6;
const MAX_HISTORY = 6;

const SVG_KEYS = new Set(["Backspace", "Enter", "Tab", "Space", "CapsLock"]);
const ARROW_GLYPHS = new Set(["", "", "", ""]);

const ALIASES: Record<string, string> = {
	PageUp: "PgUp",
	PageDown: "PgDn",
	Insert: "Ins",
	PrintScreen: "PrtSc",
	ScrollLock: "ScrLk",
	NumLock: "Num",
	Escape: "Esc",
	Delete: "Del",
};

let keyHistory = $state<StoredKey[]>([]);
let activeModifiers = $state<ModifierState>({ shift: false, ctrl: false, alt: false, super_key: false });
let capsLockOn = $state(false);
let nextKeyId = 0;
let theme = $state<Theme>({
	mode: "full",
	active_bg: "#000000",
	active_fg: "#ffffff",
	key_text: "#ffffff",
	background: "#000000",
	border: "#000000",
	...initialTheme,
});

function themeVariables(value: Theme): string {
	return [
		`--active-bg:${value.active_bg}`,
		`--active-fg:${value.active_fg}`,
		`--key-text:${value.key_text}`,
		`--background:${value.background}`,
		`--border:${value.border}`,
	].join(";");
}

function displayKey({ key }: StoredKey): string {
	if (ALIASES[key]) return ALIASES[key];
	if (/^[A-Z]$/.test(key)) return capsLockOn ? key.toUpperCase() : key.toLowerCase();
	return key;
}


function getDisplayKeys(): StoredKey[] {
	const result: StoredKey[] = [];
	let count = 0;
	for (const k of keyHistory) {
		result.push(k);
		count += 1;
		if (theme.mode === "keys_only") {
			count += +k.shift + +k.ctrl + +k.alt + +k.super;
		}
		if (count >= MAX_DISPLAY_UNITS) break;
	}
	return result.reverse();
}

onMount(() => {
	let mounted = true;

	const unlistenTheme = listen<Theme>("theme-changed", ({ payload }) => {
		theme = payload;
	});
	const unlistenKeys = listen<KeyEvent>("key-event", ({ payload: { key, modifiers } }) => {
		if (key === "CapsLock") capsLockOn = !capsLockOn;
		activeModifiers = modifiers;

		const entry: StoredKey = {
			id: nextKeyId++,
			key,
			shift: modifiers.shift,
			ctrl: modifiers.ctrl,
			alt: modifiers.alt,
			super: modifiers.super_key,
		};

		const isAlias = key in ALIASES;
		const prevWasAlias = keyHistory.length > 0 && keyHistory[0].key in ALIASES;
		keyHistory = (isAlias || prevWasAlias)
			? [entry]
			: [entry, ...keyHistory].slice(0, MAX_HISTORY);
	});

	Promise.all([unlistenTheme, unlistenKeys])
		.then(() => invoke<Theme>("get_theme"))
		.then(value => {
			if (!mounted) return;
			theme = value;
		})
		.catch(error => console.error("Could not initialize KeyRC window", error));

	return () => {
		mounted = false;
		unlistenTheme.then(u => u());
		unlistenKeys.then(u => u());
	};
});
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<main
	onmousedown={() => getCurrentWindow().startDragging()}
	class="theme-root w-full h-screen cursor-move flex flex-col justify-center items-center gap-0.5"
	style={themeVariables(theme)}
>
	<div
		class="key-display h-[70px] shrink-0 w-full flex flex-row justify-center items-center rounded-tr-3xl rounded-tl-3xl overflow-hidden gap-1"
		class:keys-only={theme.mode === "keys_only"}
	>
			{#each getDisplayKeys() as stored (stored.id)}
				<span class="key-token">
				{#if theme.mode === "keys_only" && stored.super}
<svg xmlns="http://www.w3.org/2000/svg" style="font-size:{FONT_SIZE}px" width="1em" height="1em" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15.012 5.977v12.046c0 2.645 3.316 3.954 5.14 2.13c1.825-1.825.516-5.141-2.13-5.141H5.978c-2.645 0-3.953 3.316-2.13 5.14c1.825 1.825 5.142.516 5.142-2.13V5.978c0-2.645-3.317-3.953-5.141-2.13c-1.824 1.825-.516 5.142 2.13 5.142h12.045c2.645 0 3.954-3.317 2.13-5.141s-5.141-.516-5.141 2.13"/></svg>
				{/if}
				{#if theme.mode === "keys_only" && stored.ctrl}
<svg xmlns="http://www.w3.org/2000/svg" style="font-size:{FONT_SIZE}px" width="1em" height="1em" viewBox="0 0 16 16"><path fill="currentColor" d="M11.5 7a.5.5 0 0 1-.377-.171l-3.124-3.57l-3.124 3.57a.5.5 0 1 1-.753-.659l3.5-4a.502.502 0 0 1 .752 0l3.5 4a.5.5 0 0 1-.376.83z"/></svg>
				{/if}
				{#if theme.mode === "keys_only" && stored.alt}
<svg xmlns="http://www.w3.org/2000/svg" style="font-size:{FONT_SIZE}px" width="1em" height="1em" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 5.25h5.625l6.75 13.5H21m-6.75-13.5H21"/></svg>
				{/if}
				{#if theme.mode === "keys_only" && stored.shift}
<svg xmlns="http://www.w3.org/2000/svg" style="font-size:{FONT_SIZE}px" width="1em" height="1em" viewBox="0 0 16 16"><path fill="currentColor" d="M7.27 2.047a1 1 0 0 1 1.46 0l6.345 6.77c.6.638.146 1.683-.73 1.683H11.5v3a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-3H1.654C.78 10.5.326 9.455.924 8.816zM14.346 9.5L8 2.731L1.654 9.5H4.5a1 1 0 0 1 1 1v3h5v-3a1 1 0 0 1 1-1z"/></svg>
				{/if}
				{#if SVG_KEYS.has(stored.key)}
				{#if stored.key === "Backspace"}
<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" style="font-size:{FONT_SIZE}px" viewBox="0 0 24 24"><path fill="currentColor" d="m11.4 16l2.6-2.6l2.6 2.6l1.4-1.4l-2.6-2.6L18 9.4L16.6 8L14 10.6L11.4 8L10 9.4l2.6 2.6l-2.6 2.6zM9 20q-.475 0-.9-.213t-.7-.587L2 12l5.4-7.2q.275-.375.7-.587T9 4h11q.825 0 1.413.587T22 6v12q0 .825-.587 1.413T20 20zm-4.5-8L9 18h11V6H9zm10 0"/></svg>
				{:else if stored.key === "Enter"}
<svg xmlns="http://www.w3.org/2000/svg" style="font-size:{FONT_SIZE}px" width="1em" height="1em" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="square" stroke-width="2" d="M5.75 16H16a3 3 0 0 0 3-3V5M8 12.5L4.5 16L8 19.5"/></svg>
				{:else if stored.key === "Tab"}
<svg xmlns="http://www.w3.org/2000/svg" style="font-size:{FONT_SIZE}px" width="1em" height="1em" viewBox="0 0 16 16"><path fill="currentColor" d="m10.78 8.53l-3.75 3.75a.749.749 0 1 1-1.06-1.06l2.469-2.47H1.75a.75.75 0 0 1 0-1.5h6.689L5.97 4.78a.749.749 0 1 1 1.06-1.06l3.75 3.75a.75.75 0 0 1 0 1.06M13 12.25v-8.5a.75.75 0 0 1 1.5 0v8.5a.75.75 0 0 1-1.5 0"/></svg>
				{:else if stored.key === "Space"}
<svg xmlns="http://www.w3.org/2000/svg" style="font-size:{FONT_SIZE}px" width="1em" height="1em" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 10v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3"/></svg>
				{:else if stored.key === "CapsLock"}
<svg xmlns="http://www.w3.org/2000/svg" style="font-size:{FONT_SIZE}px" width="1em" height="1em" viewBox="0 0 56 56"><path fill="currentColor" d="M20.781 37.621h14.461c3.281 0 5.016-1.922 5.016-5.016v-4.148h8.882c1.946 0 3.493-1.148 3.493-2.953c0-1.102-.563-1.969-1.617-2.883L30.906 4.88c-.96-.844-1.851-1.406-2.906-1.406c-1.031 0-1.922.562-2.883 1.406L4.984 22.645c-1.101.96-1.617 1.757-1.617 2.859c0 1.805 1.547 2.953 3.516 2.953h8.86v4.148c0 3.094 1.757 5.016 5.038 5.016m.375-3.539c-.89 0-1.5-.586-1.5-1.477v-6.89c0-.563-.21-.797-.773-.797H8.664c-.164 0-.234-.07-.234-.187a.33.33 0 0 1 .14-.282L27.508 7.996c.21-.187.328-.258.492-.258s.305.07.492.258L47.453 24.45a.33.33 0 0 1 .14.281c0 .118-.093.188-.257.188H37.14c-.563 0-.774.234-.774.797v6.89c0 .868-.656 1.477-1.5 1.477Zm-1.383 18.445h16.29c2.695 0 4.242-1.5 4.242-4.218v-3.375c0-2.72-1.547-4.266-4.243-4.266H19.773c-2.718 0-4.265 1.57-4.265 4.266v3.375c0 2.695 1.547 4.218 4.265 4.218m.54-3.304c-.82 0-1.266-.422-1.266-1.242v-2.72c0-.82.445-1.288 1.265-1.288h15.211c.797 0 1.242.468 1.242 1.289v2.718c0 .82-.445 1.243-1.242 1.243Z"/></svg>
				{/if}
			{:else}
					<span style="font-size:{ARROW_GLYPHS.has(stored.key) ? ARROW_FONT_SIZE : FONT_SIZE}px">{displayKey(stored)}</span>
			{/if}
			</span>
		{/each}
	</div>
	{#if theme.mode === "full"}
	<div class="h-[42px] shrink-0 grid grid-cols-4 gap-0.5 w-full rounded-br-3xl rounded-bl-3xl overflow-hidden">
		<div class="modifier flex items-center justify-center" class:active={activeModifiers.shift}>
			<svg xmlns="http://www.w3.org/2000/svg" width="1.6em" height="1.4em" viewBox="0 0 16 16"><path fill="currentColor" d="M7.27 2.047a1 1 0 0 1 1.46 0l6.345 6.77c.6.638.146 1.683-.73 1.683H11.5v3a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-3H1.654C.78 10.5.326 9.455.924 8.816zM14.346 9.5L8 2.731L1.654 9.5H4.5a1 1 0 0 1 1 1v3h5v-3a1 1 0 0 1 1-1z"/></svg>
		</div>
		<div class="modifier flex items-center justify-center" class:active={activeModifiers.ctrl}>
			<svg xmlns="http://www.w3.org/2000/svg" width="1.8em" height="1.8em" viewBox="0 0 16 16"><path fill="currentColor" d="M11.5 7a.5.5 0 0 1-.377-.171l-3.124-3.57l-3.124 3.57a.5.5 0 1 1-.753-.659l3.5-4a.502.502 0 0 1 .752 0l3.5 4a.5.5 0 0 1-.376.83z"/></svg>
		</div>
		<div class="modifier flex items-center justify-center" class:active={activeModifiers.alt}>
			<svg xmlns="http://www.w3.org/2000/svg" width="1.6em" height="1.4em" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 5.25h5.625l6.75 13.5H21m-6.75-13.5H21"/></svg>
		</div>
		<div class="modifier flex items-center justify-center" class:active={activeModifiers.super_key}>
			<svg xmlns="http://www.w3.org/2000/svg" width="1.6em" height="1.4em" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15.012 5.977v12.046c0 2.645 3.316 3.954 5.14 2.13c1.825-1.825.516-5.141-2.13-5.141H5.978c-2.645 0-3.953 3.316-2.13 5.14c1.825 1.825 5.142.516 5.142-2.13V5.978c0-2.645-3.317-3.953-5.141-2.13c-1.824 1.825-.516 5.142 2.13 5.142h12.045c2.645 0 3.954-3.317 2.13-5.141s-5.141-.516-5.141 2.13"/></svg>
		</div>
	</div>
	{/if}
</main>

<style>
	.theme-root {
		color: var(--key-text);
	}

	.key-display,
	.modifier {
		background-color: var(--background);
		box-shadow: inset 0 0 0 1px var(--border);
	}

	.key-display.keys-only {
		border-radius: 1.5rem;
	}

	.key-token {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		transform-origin: center;
		animation: key-appear 140ms ease-out both;
	}

	@keyframes key-appear {
		from {
			opacity: 0;
			transform: translateY(4px) scale(0.92);
		}
		to {
			opacity: 1;
			transform: translateY(0) scale(1);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.key-token {
			animation: none;
		}
	}

	.modifier {
		color: color-mix(in srgb, var(--key-text) 35%, transparent);
		transition: color 120ms ease, background-color 120ms ease;
	}

	.modifier:first-child {
		border-bottom-left-radius: 1.5rem;
	}

	.modifier:last-child {
		border-bottom-right-radius: 1.5rem;
	}

	.modifier.active {
		background-color: var(--active-bg);
		color: var(--active-fg);
	}
</style>
