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
let isBold = $state(false);
let isItalic = $state(false);
let isUnderline = $state(false);
let isStrike = $state(false);
let isHighlight = $state(false);

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

const colorRegex = /(#[0-9A-Fa-f]{3,8}|rgb\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*\)|rgba\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*,\s*[\d.]+\s*\)|hsl\(\s*\d+\s*,\s*\d+%?\s*,\s*\d+%?\s*\)|hsla\(\s*\d+\s*,\s*\d+%?\s*,\s*\d+%?\s*,\s*[\d.]+\s*\))/gi;

function formatText(text: string): string {
	// First, detect colors, mentions, hashtags BEFORE escaping
	// Use placeholders to protect them
	const colorPlaceholders: string[] = [];
	const mentionPlaceholders: string[] = [];
	const hashtagPlaceholders: string[] = [];
	
	// Protect colors
	let processed = text.replace(colorRegex, (match) => {
		colorPlaceholders.push(match);
		return `__COLOR_${colorPlaceholders.length - 1}__`;
	});
	
	// Protect mentions
	processed = processed.replace(/@(\w+)/g, (match, name) => {
		mentionPlaceholders.push(name);
		return `__MENTION_${mentionPlaceholders.length - 1}__`;
	});
	
	// Protect hashtags (but not HTML color codes)
	processed = processed.replace(/#(\w+)/g, (match, tag) => {
		// Skip if it looks like a hex color
		if (/^[0-9A-Fa-f]{3,8}$/.test(tag)) return match;
		hashtagPlaceholders.push(tag);
		return `__HASHTAG_${hashtagPlaceholders.length - 1}__`;
	});
	
	// Now escape HTML
	let formatted = processed
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;');
	
	// Bold: <b>text</b>
	formatted = formatted.replace(/&lt;b&gt;(.+?)&lt;\/b&gt;/g, '<strong>$1</strong>');
	
	// Italic: <i>text</i>
	formatted = formatted.replace(/&lt;i&gt;(.+?)&lt;\/i&gt;/g, '<em>$1</em>');
	
	// Underline: <u>text</u>
	formatted = formatted.replace(/&lt;u&gt;(.+?)&lt;\/u&gt;/g, '<u>$1</u>');
	
	// Strikethrough: <s>text</s>
	formatted = formatted.replace(/&lt;s&gt;(.+?)&lt;\/s&gt;/g, '<s>$1</s>');
	
	// Highlight: <h>text</h>
	formatted = formatted.replace(/&lt;h&gt;(.+?)&lt;\/h&gt;/g, '<mark style="background:#fde047;color:#000;padding:1px 4px;border-radius:2px;">$1</mark>');
	
	// Restore colors with formatting
	colorPlaceholders.forEach((color, i) => {
		formatted = formatted.replace(`__COLOR_${i}__`, `<span style="display:inline-flex;align-items:center;gap:3px;"><span style="display:inline-block;width:16px;height:16px;border-radius:3px;background:${color};border:1px solid rgba(255,255,255,0.3);vertical-align:middle;"></span>${color}</span>`);
	});
	
	// Restore mentions with formatting
	mentionPlaceholders.forEach((name, i) => {
		formatted = formatted.replace(`__MENTION_${i}__`, `<span style="color:#60a5fa;">@${name}</span>`);
	});
	
	// Restore hashtags with formatting
	hashtagPlaceholders.forEach((tag, i) => {
		formatted = formatted.replace(`__HASHTAG_${i}__`, `<span style="color:#a78bfa;">#${tag}</span>`);
	});
	
	// Preserve newlines
	formatted = formatted.replace(/\n/g, '<br>');
	
	return formatted;
}

function wrapWithFormat(char: string): string {
	let result = char;
	if (isBold) result = `<b>${result}</b>`;
	if (isItalic) result = `<i>${result}</i>`;
	if (isUnderline) result = `<u>${result}</u>`;
	if (isStrike) result = `<s>${result}</s>`;
	if (isHighlight) result = `<h>${result}</h>`;
	return result;
}

function mergeFormatting(text: string): string {
	// Merge adjacent same tags
	return text
		.replace(/<\/b><b>/g, '')
		.replace(/<\/i><i>/g, '')
		.replace(/<\/u><u>/g, '')
		.replace(/<\/s><s>/g, '')
		.replace(/<\/h><h>/g, '');
}

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
		const ctrl = modifiers.includes("Ctrl");

		if (key === "Escape") {
			getCurrentWindow().close();
			return;
		}

		// Ctrl+B to toggle bold
		if (ctrl && key === "B") {
			isBold = !isBold;
			return;
		}

		// Ctrl+I to toggle italic
		if (ctrl && key === "I") {
			isItalic = !isItalic;
			return;
		}

		// Ctrl+U to toggle underline
		if (ctrl && key === "U") {
			isUnderline = !isUnderline;
			return;
		}

		// Ctrl+S to toggle strikethrough
		if (ctrl && key === "S") {
			isStrike = !isStrike;
			return;
		}

		// Ctrl+L to toggle highlight
		if (ctrl && key === "L") {
			isHighlight = !isHighlight;
			return;
		}

		if (key === "Enter") {
			if (shift) {
				currentText += wrapWithFormat("\n");
				currentText = mergeFormatting(currentText);
			} else if (currentText.trim()) {
				messages = [...messages, { text: currentText.trim(), time: getTimeString() }].slice(-maxMessages);
				currentText = "";
				isBold = false;
				isItalic = false;
				isUnderline = false;
				isStrike = false;
				isHighlight = false;
			}
			return;
		}

		if (key === "Backspace") {
			if (!currentText) return;
			
			// Handle single tags: <b>, <i>, <u>, <s>, <h>
			const tagPatterns = ['b', 'i', 'u', 's', 'h'];
			for (const tag of tagPatterns) {
				const regex = new RegExp(`<${tag}>(.+)</${tag}>$`);
				const match = currentText.match(regex);
				if (match) {
					const content = match[1].slice(0, -1);
					currentText = currentText.replace(regex, content ? `<${tag}>${content}</${tag}>` : '');
					return;
				}
			}
			
			// Remove last plain character
			currentText = currentText.slice(0, -1);
			return;
		}

		if (key === "Space") {
			currentText += wrapWithFormat(" ");
			currentText = mergeFormatting(currentText);
			return;
		}

		if (key === "Tab") {
			currentText += wrapWithFormat("    ");
			currentText = mergeFormatting(currentText);
			return;
		}

		// Skip other special keys
		if (key.length > 1) return;

		// Handle shifted keys
		let char: string;
		if (shift && shiftedKeys[key]) {
			char = shiftedKeys[key];
		} else if (shift) {
			char = key.toUpperCase();
		} else {
			char = key.toLowerCase();
		}
		currentText += wrapWithFormat(char);
		currentText = mergeFormatting(currentText);
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
				class="inline-block w-fit max-w-[85%] px-6 py-3 rounded-3xl font-medium"
				style="background-color: {theme.bg_dark}; color: {theme.fg}; border: 1px solid {theme.border}; font-size: {chatFontSize}px;"
			>
				{@html formatText(message.text)}
				{#if showTimestamp}
					<div class="text-xs opacity-40 mt-1" style="font-size: {Math.max(chatFontSize - 6, 10)}px;">{message.time}</div>
				{/if}
			</div>
		{/each}
	</div>

	{#if currentText}
		<div 
			class="inline-block w-fit max-w-[85%] px-6 py-3 rounded-3xl font-medium mx-4"
			style="background-color: {theme.bg_dark}; color: {theme.fg}; border: 1px solid {theme.border}; font-size: {chatFontSize}px;"
		>
			{@html formatText(currentText)}<span class="animate-pulse opacity-50">|</span>
		</div>
	{:else}
		<div class="px-4" style="color: {theme.fg}; opacity: 0.5; font-size: {chatFontSize}px;">
			Type something... (Enter to send, Shift+Enter for new line, ESC to close)
		</div>
	{/if}
	
	{#if isBold || isItalic || isUnderline || isStrike || isHighlight}
		<div class="flex gap-2 px-4 mt-2" style="color: {theme.fg}; opacity: 0.7; font-size: 12px;">
			{#if isBold}<span class="px-2 py-0.5 rounded" style="background: {theme.bg_dark}; border: 1px solid {theme.border};"><strong>B</strong></span>{/if}
			{#if isItalic}<span class="px-2 py-0.5 rounded" style="background: {theme.bg_dark}; border: 1px solid {theme.border};"><em>I</em></span>{/if}
			{#if isUnderline}<span class="px-2 py-0.5 rounded" style="background: {theme.bg_dark}; border: 1px solid {theme.border};"><u>U</u></span>{/if}
			{#if isStrike}<span class="px-2 py-0.5 rounded" style="background: {theme.bg_dark}; border: 1px solid {theme.border};"><s>S</s></span>{/if}
			{#if isHighlight}<span class="px-2 py-0.5 rounded" style="background: #fde047; color: #000;">H</span>{/if}
		</div>
	{/if}
</div>
