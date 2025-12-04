<script lang="ts">
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize, LogicalPosition } from "@tauri-apps/api/window";
import { onMount } from "svelte";
import { readTextFile } from "@tauri-apps/plugin-fs";
import { homeDir } from "@tauri-apps/api/path";

interface KeyEvent {
	key: string;
	modifiers: string[];
	event_type: string;
}

interface Theme {
	bg: string;
	fg: string;
	bg_dark: string;
	border: string;
}

const defaultTheme: Theme = {
	bg: "#111827",
	fg: "#F3F4F6",
	bg_dark: "#0b1221",
	border: "#1e2534",
};

interface Message {
	text: string;
	time: string;
}

let messages = $state<Message[]>([]);
let maxMessages = $state(8);

function calculateMaxMessages() {
	const screenHeight = window.screen.height;
	// padding: p-8 = 32px * 2 = 64px
	// input area + hint text: ~70px
	// mb-4 = 16px
	// extra buffer for multi-line messages: 150px
	const padding = 64 + 70 + 16 + 150;
	// gap-3 = 12px
	const gap = 12;
	// message: py-3 (24px) + text lines (~fontSize * 1.4 * 2 for avg multi-line) + timestamp if shown + border
	const timestampHeight = showTimestamp ? (Math.max(chatFontSize - 6, 10) * 1.4 + 4) : 0;
	const messageHeight = 24 + (chatFontSize * 1.4 * 2) + timestampHeight + 2 + gap;
	const availableHeight = screenHeight - padding;
	maxMessages = Math.max(1, Math.floor(availableHeight / messageHeight));
	// Trim existing messages if they exceed the new limit
	if (messages.length > maxMessages) {
		messages = messages.slice(-maxMessages);
	}
}

function getTimeString(): string {
	const now = new Date();
	return now.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
}
let currentText = $state("");
// Load cached theme from localStorage for instant display
function getCachedTheme(): Theme {
	try {
		const cached = localStorage.getItem("keyrc-theme-cache");
		if (cached) return JSON.parse(cached);
	} catch {}
	return { ...defaultTheme };
}

let theme = $state<Theme>(getCachedTheme());
let chatFontSize = $state(18);
let showTimestamp = $state(true);
let chatBgOpacity = $state(100);
let chatBgImage = $state("");

const shiftedKeys: Record<string, string> = {
	"1": "!", "2": "@", "3": "#", "4": "$", "5": "%",
	"6": "^", "7": "&", "8": "*", "9": "(", "0": ")",
	"-": "_", "=": "+", "[": "{", "]": "}", "\\": "|",
	";": ":", "'": "\"", ",": "<", ".": ">", "/": "?",
	"`": "~",
};

async function loadTheme() {
	try {
		const home = await homeDir();
		const content = await readTextFile(`${home}/.config/keyrc/theme`);
		const lines = content.split("\n");
		const colors: Record<string, string> = {};
		for (const line of lines) {
			const [key, value] = line.split("=");
			if (key && value) {
				colors[key.trim()] = value.trim();
			}
		}
		theme = {
			bg: colors.bg || defaultTheme.bg,
			fg: colors.fg || defaultTheme.fg,
			bg_dark: colors.bg_dark || defaultTheme.bg_dark,
			border: colors.border || defaultTheme.border,
		};
		localStorage.setItem("keyrc-theme-cache", JSON.stringify(theme));
	} catch {
		theme = { ...defaultTheme };
	}
}

function loadSettings() {
	const saved = localStorage.getItem("keyrc-settings");
	if (saved) {
		const settings = JSON.parse(saved);
		chatFontSize = settings.chatFontSize ?? 18;
		showTimestamp = settings.showTimestamp ?? true;
		chatBgOpacity = settings.chatBgOpacity ?? 100;
		chatBgImage = settings.chatBgImage ?? "";
	}
	calculateMaxMessages();
}

onMount(() => {
	loadTheme();
	loadSettings();
	
	// Set window to screen size immediately
	const win = getCurrentWindow();
	const screenWidth = window.screen.width;
	const screenHeight = window.screen.height;
	win.setPosition(new LogicalPosition(0, 0));
	win.setSize(new LogicalSize(screenWidth, screenHeight));
	win.setFullscreen(true);

	const unlistenPromise = listen<KeyEvent>("key-event", (event) => {
		const { key, modifiers } = event.payload;
		const shift = modifiers.includes("Shift");

		if (key === "Escape") {
			getCurrentWindow().close();
			return;
		}

		if (key === "Enter") {
			if (shift) {
				currentText += "\n";
			} else if (currentText.trim()) {
				messages = [...messages, { text: currentText.trim(), time: getTimeString() }].slice(-maxMessages);
				currentText = "";
			}
			return;
		}

		if (key === "Backspace") {
			currentText = currentText.slice(0, -1);
			return;
		}

		if (key === "Space") {
			currentText += " ";
			return;
		}

		if (key === "Tab") {
			currentText += "    ";
			return;
		}

		// Skip other special keys
		if (key.length > 1) return;

		// Handle shifted keys
		if (shift && shiftedKeys[key]) {
			currentText += shiftedKeys[key];
		} else if (shift) {
			currentText += key.toUpperCase();
		} else {
			currentText += key.toLowerCase();
		}
	});

	return () => {
		unlistenPromise.then((unlisten) => unlisten());
	};
});
</script>

<div
	class="fixed inset-0 flex flex-col justify-end p-8"
	style="background-color: {theme.bg}{Math.round(chatBgOpacity * 2.55).toString(16).padStart(2, '0')}; {chatBgImage ? `background-image: url('${chatBgImage}'); background-size: cover; background-position: center;` : ''}"
>
	<div class="flex flex-col gap-3 mb-4 px-4">
		{#each messages as message}
			<div 
				class="inline-block w-fit max-w-[85%] px-6 py-3 rounded-3xl font-medium whitespace-pre-wrap"
				style="background-color: {theme.bg_dark}; color: {theme.fg}; border: 1px solid {theme.border}; font-size: {chatFontSize}px;"
			>
				{message.text}
				{#if showTimestamp}
					<div class="text-xs opacity-40 mt-1" style="font-size: {Math.max(chatFontSize - 6, 10)}px;">{message.time}</div>
				{/if}
			</div>
		{/each}
	</div>

	{#if currentText}
		<div 
			class="inline-block w-fit max-w-[85%] px-6 py-3 rounded-3xl font-medium mx-4 whitespace-pre-wrap"
			style="background-color: {theme.bg_dark}; color: {theme.fg}; border: 1px solid {theme.border}; font-size: {chatFontSize}px;"
		>
			{currentText}<span class="animate-pulse opacity-50">|</span>
		</div>
	{:else}
		<div class="px-4" style="color: {theme.fg}; opacity: 0.5; font-size: {chatFontSize}px;">
			Type something... (Enter to send, Shift+Enter for new line, ESC to close)
		</div>
	{/if}
</div>
