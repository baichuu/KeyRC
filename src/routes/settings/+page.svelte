<script lang="ts">
import { onMount } from "svelte";
import { readTextFile } from "@tauri-apps/plugin-fs";
import { homeDir } from "@tauri-apps/api/path";

interface Settings {
	fontSize: number;
	fontFamily: string;
	textColor: string;
	chatFontSize: number;
	showTimestamp: boolean;
	chatBgOpacity: number;
	showKeys: boolean;
	chatBgImage: string;
	chatBgImageTab: "url" | "upload";
	enableSound: boolean;
}

interface Theme {
	bg: string;
	fg: string;
	bg_dark: string;
	sel_bg: string;
	border: string;
	blue: string;
	grey: string;
}

const defaultTheme: Theme = {
	bg: "#111827",
	fg: "#F3F4F6",
	bg_dark: "#0b1221",
	sel_bg: "#282f3e",
	border: "#1e2534",
	blue: "#A5B4FC",
	grey: "#5f6675",
};

const fonts = [
	{ label: "Roboto Mono", value: "Roboto Mono" },
	{ label: "JetBrains Mono", value: "JetBrains Mono" },
];

const presetColors = [
	"#ffffff", "#f87171", "#fb923c", "#facc15", 
	"#4ade80", "#22d3ee", "#60a5fa", "#a78bfa",
	"#f472b6", "#9ca3af", "#000000", "#1e293b",
];

const defaultSettings: Settings = {
	fontSize: 48,
	fontFamily: "Roboto Mono",
	textColor: "#ffffff",
	chatFontSize: 18,
	showTimestamp: true,
	chatBgOpacity: 100,
	showKeys: true,
	chatBgImage: "",
	chatBgImageTab: "url",
	enableSound: false,
};

let settings = $state<Settings>({ ...defaultSettings });
let theme = $state<Theme>({ ...defaultTheme });

function loadSettings() {
	const saved = localStorage.getItem("keyrc-settings");
	if (saved) {
		settings = { ...defaultSettings, ...JSON.parse(saved) };
	}
}

function saveSettings() {
	localStorage.setItem("keyrc-settings", JSON.stringify(settings));
}

function handleImageUpload(event: Event) {
	const input = event.target as HTMLInputElement;
	const file = input.files?.[0];
	if (file) {
		const reader = new FileReader();
		reader.onload = (e) => {
			settings.chatBgImage = e.target?.result as string;
			saveSettings();
		};
		reader.readAsDataURL(file);
	}
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
			sel_bg: colors.sel_bg || defaultTheme.sel_bg,
			border: colors.border || defaultTheme.border,
			blue: colors.blue || defaultTheme.blue,
			grey: colors.grey || defaultTheme.grey,
		};
	} catch {
		theme = { ...defaultTheme };
	}
}

onMount(() => {
	loadSettings();
	loadTheme();

	const themeInterval = setInterval(loadTheme, 1000);

	return () => {
		clearInterval(themeInterval);
	};
});
</script>

<div class="min-h-screen p-8 overflow-auto" style="background-color: {theme.bg}; color: {theme.fg};">
	<div class="space-y-6">
		<div>
			<label class="block text-sm mb-2" style="color: {theme.grey};">Font Family</label>
			<div class="grid grid-cols-2 gap-2">
				{#each fonts as font}
					<button
						onclick={() => { settings.fontFamily = font.value; saveSettings(); }}
						class="px-3 py-2 rounded border"
						style="font-family: '{font.value}', monospace; background-color: {settings.fontFamily === font.value ? theme.blue : theme.bg_dark}; border-color: {settings.fontFamily === font.value ? theme.blue : theme.border}; color: {theme.fg};"
					>
						{font.label}
					</button>
				{/each}
			</div>
		</div>

		<div>
			<label class="block text-sm mb-2" style="color: {theme.grey};">Font Size: {settings.fontSize}px</label>
			<input
				type="range"
				min="24"
				max="72"
				bind:value={settings.fontSize}
				oninput={saveSettings}
				class="w-full h-2 rounded-lg appearance-none cursor-pointer"
				style="background: linear-gradient(to right, {theme.blue} 0%, {theme.blue} {((settings.fontSize - 24) / 48) * 100}%, {theme.bg_dark} {((settings.fontSize - 24) / 48) * 100}%, {theme.bg_dark} 100%); --thumb-bg: {theme.bg}; --thumb-border: {theme.blue};"
			/>
		</div>

		<div>
			<label class="block text-sm mb-2" style="color: {theme.grey};">Text Color</label>
			<div class="flex flex-wrap gap-1.5 mb-3">
				{#each presetColors as color}
					<button
						onclick={() => { settings.textColor = color; saveSettings(); }}
						class="w-6 h-6 rounded border-2"
						style="background-color: {color}; border-color: {settings.textColor === color ? theme.blue : theme.border};"
					></button>
				{/each}
			</div>
			<input
				type="text"
				bind:value={settings.textColor}
				oninput={saveSettings}
				class="w-full rounded px-3 py-2 uppercase"
				style="background-color: {theme.bg_dark}; border: 1px solid {theme.border}; color: {theme.fg};"
				placeholder="#ffffff"
			/>
		</div>

		<div>
			<label class="block text-sm mb-2" style="color: {theme.grey};">Chat Font Size: {settings.chatFontSize}px</label>
			<input
				type="range"
				min="14"
				max="32"
				bind:value={settings.chatFontSize}
				oninput={saveSettings}
				class="w-full h-2 rounded-lg appearance-none cursor-pointer"
				style="background: linear-gradient(to right, {theme.blue} 0%, {theme.blue} {((settings.chatFontSize - 14) / 18) * 100}%, {theme.bg_dark} {((settings.chatFontSize - 14) / 18) * 100}%, {theme.bg_dark} 100%); --thumb-bg: {theme.bg}; --thumb-border: {theme.blue};"
			/>
		</div>

		<div class="flex items-center justify-between">
			<label class="text-sm" style="color: {theme.grey};">Show Timestamps in Chat</label>
			<label class="switch" style="--switch-bg: {theme.grey}; --switch-checked-bg: {theme.blue}; --icon-cross-color: {theme.grey}; --icon-checkmark-color: {theme.blue};">
				<input type="checkbox" bind:checked={settings.showTimestamp} onchange={saveSettings} />
				<div class="slider">
					<div class="circle">
						<svg class="cross" viewBox="0 0 365.696 365.696" height="6" width="6" xmlns="http://www.w3.org/2000/svg">
							<path fill="currentColor" d="M243.188 182.86 356.32 69.726c12.5-12.5 12.5-32.766 0-45.247L341.238 9.398c-12.504-12.503-32.77-12.503-45.25 0L182.86 122.528 69.727 9.374c-12.5-12.5-32.766-12.5-45.247 0L9.375 24.457c-12.5 12.504-12.5 32.77 0 45.25l113.152 113.152L9.398 295.99c-12.503 12.503-12.503 32.769 0 45.25L24.48 356.32c12.5 12.5 32.766 12.5 45.247 0l113.132-113.132L295.99 356.32c12.503 12.5 32.769 12.5 45.25 0l15.081-15.082c12.5-12.504 12.5-32.77 0-45.25zm0 0"></path>
						</svg>
						<svg class="checkmark" viewBox="0 0 24 24" height="10" width="10" xmlns="http://www.w3.org/2000/svg">
							<path fill="currentColor" d="M9.707 19.121a.997.997 0 0 1-1.414 0l-5.646-5.647a1.5 1.5 0 0 1 0-2.121l.707-.707a1.5 1.5 0 0 1 2.121 0L9 14.171l9.525-9.525a1.5 1.5 0 0 1 2.121 0l.707.707a1.5 1.5 0 0 1 0 2.121z"></path>
						</svg>
					</div>
				</div>
			</label>
		</div>

		<div class="flex items-center justify-between">
			<label class="text-sm" style="color: {theme.grey};">Enable Keyboard Sound</label>
			<label class="switch" style="--switch-bg: {theme.grey}; --switch-checked-bg: {theme.blue}; --icon-cross-color: {theme.grey}; --icon-checkmark-color: {theme.blue};">
				<input type="checkbox" bind:checked={settings.enableSound} onchange={saveSettings} />
				<div class="slider">
					<div class="circle">
						<svg class="cross" viewBox="0 0 365.696 365.696" height="6" width="6" xmlns="http://www.w3.org/2000/svg">
							<path fill="currentColor" d="M243.188 182.86 356.32 69.726c12.5-12.5 12.5-32.766 0-45.247L341.238 9.398c-12.504-12.503-32.77-12.503-45.25 0L182.86 122.528 69.727 9.374c-12.5-12.5-32.766-12.5-45.247 0L9.375 24.457c-12.5 12.504-12.5 32.77 0 45.25l113.152 113.152L9.398 295.99c-12.503 12.503-12.503 32.769 0 45.25L24.48 356.32c12.5 12.5 32.766 12.5 45.247 0l113.132-113.132L295.99 356.32c12.503 12.5 32.769 12.5 45.25 0l15.081-15.082c12.5-12.504 12.5-32.77 0-45.25zm0 0"></path>
						</svg>
						<svg class="checkmark" viewBox="0 0 24 24" height="10" width="10" xmlns="http://www.w3.org/2000/svg">
							<path fill="currentColor" d="M9.707 19.121a.997.997 0 0 1-1.414 0l-5.646-5.647a1.5 1.5 0 0 1 0-2.121l.707-.707a1.5 1.5 0 0 1 2.121 0L9 14.171l9.525-9.525a1.5 1.5 0 0 1 2.121 0l.707.707a1.5 1.5 0 0 1 0 2.121z"></path>
						</svg>
					</div>
				</div>
			</label>
		</div>

		<div>
			<label class="block text-sm mb-2" style="color: {theme.grey};">Chat Background Opacity: {settings.chatBgOpacity}%</label>
			<input
				type="range"
				min="0"
				max="100"
				bind:value={settings.chatBgOpacity}
				oninput={saveSettings}
				class="w-full h-2 rounded-lg appearance-none cursor-pointer"
				style="background: linear-gradient(to right, {theme.blue} 0%, {theme.blue} {settings.chatBgOpacity}%, {theme.bg_dark} {settings.chatBgOpacity}%, {theme.bg_dark} 100%); --thumb-bg: {theme.bg}; --thumb-border: {theme.blue};"
			/>
		</div>

		<div>
			<label class="block text-sm mb-2" style="color: {theme.grey};">Chat Background Image</label>
			<div class="flex mb-2 rounded overflow-hidden" style="border: 1px solid {theme.border};">
				<button
					onclick={() => { settings.chatBgImageTab = "url"; saveSettings(); }}
					class="flex-1 py-2 text-sm"
					style="background-color: {settings.chatBgImageTab === 'url' ? theme.blue : theme.bg_dark}; color: {settings.chatBgImageTab === 'url' ? theme.bg : theme.fg};"
				>
					URL
				</button>
				<button
					onclick={() => { settings.chatBgImageTab = "upload"; saveSettings(); }}
					class="flex-1 py-2 text-sm"
					style="background-color: {settings.chatBgImageTab === 'upload' ? theme.blue : theme.bg_dark}; color: {settings.chatBgImageTab === 'upload' ? theme.bg : theme.fg};"
				>
					Upload
				</button>
			</div>
			{#if settings.chatBgImageTab === "url"}
				<input
					type="text"
					bind:value={settings.chatBgImage}
					oninput={saveSettings}
					class="w-full rounded px-3 py-2"
					style="background-color: {theme.bg_dark}; border: 1px solid {theme.border}; color: {theme.fg};"
					placeholder="https://example.com/image.jpg"
				/>
			{:else}
				<input
					type="file"
					accept="image/*"
					onchange={handleImageUpload}
					class="w-full rounded px-3 py-2 cursor-pointer"
					style="background-color: {theme.bg_dark}; border: 1px solid {theme.border}; color: {theme.fg};"
				/>
			{/if}
			{#if settings.chatBgImage}
				<button
					onclick={() => { settings.chatBgImage = ""; saveSettings(); }}
					class="mt-2 w-full py-2 rounded text-sm"
					style="background-color: {theme.bg_dark}; border: 1px solid {theme.border}; color: {theme.fg};"
				>
					Clear Background
				</button>
			{/if}
		</div>

		<div class="pt-6" style="border-top: 1px solid {theme.border};">
			<h2 class="text-lg font-semibold mb-4">Preview</h2>
			<div
				class="rounded-xl p-6 flex items-center justify-center bg-black/90"
				style="color: {settings.textColor}; font-family: {settings.fontFamily};"
			>
				<span style="font-size: {settings.fontSize}px;">Abc 123</span>
			</div>
		</div>

		<button
			onclick={() => { settings = { ...defaultSettings }; saveSettings(); }}
			class="w-full py-3 rounded border"
			style="background-color: {theme.bg_dark}; border-color: {theme.border}; color: {theme.fg};"
		>
			Reset to Defaults
		</button>
	</div>
</div>

<style>
	input[type="range"]::-webkit-slider-thumb {
		-webkit-appearance: none;
		appearance: none;
		width: 24px;
		height: 24px;
		border-radius: 50%;
		background: var(--thumb-bg);
		border: 2px solid var(--thumb-border);
		cursor: pointer;
	}

	input[type="range"]::-moz-range-thumb {
		width: 24px;
		height: 24px;
		border-radius: 50%;
		background: var(--thumb-bg);
		border: 2px solid var(--thumb-border);
		cursor: pointer;
	}

	.switch {
		--switch-width: 46px;
		--switch-height: 24px;
		--switch-offset: calc((var(--switch-height) - var(--circle-diameter)) / 2);
		--switch-transition: all .2s cubic-bezier(0.27, 0.2, 0.25, 1.51);
		--circle-diameter: 18px;
		--circle-bg: #fff;
		--circle-shadow: 1px 1px 2px rgba(146, 146, 146, 0.45);
		--circle-checked-shadow: -1px 1px 2px rgba(163, 163, 163, 0.45);
		--effect-width: calc(var(--circle-diameter) / 2);
		--effect-height: calc(var(--effect-width) / 2 - 1px);
		--effect-bg: var(--circle-bg);
		display: inline-block;
	}

	.switch input {
		display: none;
	}

	.switch svg {
		transition: var(--switch-transition);
		position: absolute;
	}

	.switch .checkmark {
		width: 10px;
		color: var(--icon-checkmark-color);
		transform: scale(0);
	}

	.switch .cross {
		width: 6px;
		color: var(--icon-cross-color);
	}

	.switch .slider {
		box-sizing: border-box;
		width: var(--switch-width);
		height: var(--switch-height);
		background: var(--switch-bg);
		border-radius: 999px;
		display: flex;
		align-items: center;
		position: relative;
		transition: var(--switch-transition);
		cursor: pointer;
	}

	.switch .circle {
		width: var(--circle-diameter);
		height: var(--circle-diameter);
		background: var(--circle-bg);
		border-radius: inherit;
		box-shadow: var(--circle-shadow);
		display: flex;
		align-items: center;
		justify-content: center;
		transition: var(--switch-transition);
		z-index: 1;
		position: absolute;
		left: var(--switch-offset);
	}

	.switch .slider::before {
		content: "";
		position: absolute;
		width: var(--effect-width);
		height: var(--effect-height);
		left: calc(var(--switch-offset) + (var(--effect-width) / 2));
		background: var(--effect-bg);
		border-radius: 1px;
		transition: all .2s ease-in-out;
	}

	.switch input:checked + .slider {
		background: var(--switch-checked-bg);
	}

	.switch input:checked + .slider .checkmark {
		transform: scale(1);
	}

	.switch input:checked + .slider .cross {
		transform: scale(0);
	}

	.switch input:checked + .slider::before {
		left: calc(100% - var(--effect-width) - (var(--effect-width) / 2) - var(--switch-offset));
	}

	.switch input:checked + .slider .circle {
		left: calc(100% - var(--circle-diameter) - var(--switch-offset));
		box-shadow: var(--circle-checked-shadow);
	}
</style>
