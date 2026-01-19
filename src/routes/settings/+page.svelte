<script lang="ts">
import { onMount } from "svelte";
import { readTextFile } from "@tauri-apps/plugin-fs";
import { homeDir } from "@tauri-apps/api/path";

interface Settings {
	fontSize: number;
	fontFamily: string;
	textColor: string;
	showKeys: boolean;
	windowOpacity: number;
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
	{ label: "Fira Code", value: "Fira Code" },
	{ label: "Source Code Pro", value: "Source Code Pro" },
	{ label: "Ubuntu Mono", value: "Ubuntu Mono" },
	{ label: "IBM Plex Mono", value: "IBM Plex Mono" },
	{ label: "Inconsolata", value: "Inconsolata" },
];

const presetColors = [
	"#ffffff", "#f87171", "#fb923c", "#facc15",
	"#4ade80", "#22d3ee", "#60a5fa", "#a78bfa",
	"#f472b6", "#9ca3af",
];

const defaultSettings: Settings = {
	fontSize: 36,
	fontFamily: "Roboto Mono",
	textColor: "#ffffff",
	showKeys: true,
	windowOpacity: 90,
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
				max="48"
				bind:value={settings.fontSize}
				oninput={saveSettings}
				class="w-full h-2 rounded-lg appearance-none cursor-pointer"
				style="background: linear-gradient(to right, {theme.blue} 0%, {theme.blue} {((settings.fontSize - 24) / 24) * 100}%, {theme.bg_dark} {((settings.fontSize - 24) / 24) * 100}%, {theme.bg_dark} 100%); --thumb-bg: {theme.bg}; --thumb-border: {theme.blue};"
			/>
		</div>

		<div>
			<label class="block text-sm mb-2" style="color: {theme.grey};">Text Color</label>
			<div class="flex flex-wrap gap-1.5">
				{#each presetColors as color}
					<button
						onclick={() => { settings.textColor = color; saveSettings(); }}
						class="w-6 h-6 rounded border-2"
						style="background-color: {color}; border-color: {settings.textColor === color ? theme.blue : theme.border};"
					></button>
				{/each}
			</div>
		</div>

		<div>
			<label class="block text-sm mb-2" style="color: {theme.grey};">Window Opacity: {settings.windowOpacity}%</label>
			<input
				type="range"
				min="10"
				max="100"
				bind:value={settings.windowOpacity}
				oninput={saveSettings}
				class="w-full h-2 rounded-lg appearance-none cursor-pointer"
				style="background: linear-gradient(to right, {theme.blue} 0%, {theme.blue} {((settings.windowOpacity - 10) / 90) * 100}%, {theme.bg_dark} {((settings.windowOpacity - 10) / 90) * 100}%, {theme.bg_dark} 100%); --thumb-bg: {theme.bg}; --thumb-border: {theme.blue};"
			/>
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
</style>
