<script lang="ts">
import { onMount } from "svelte";
import { readTextFile } from "@tauri-apps/plugin-fs";
import { homeDir } from "@tauri-apps/api/path";

interface Settings {
	fontSize: string;
	fontFamily: string;
	textColor: string;
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

const fontSizes = [
	{ label: "2XL", value: "text-2xl" },
	{ label: "3XL", value: "text-3xl" },
	{ label: "4XL", value: "text-4xl" },
	{ label: "5XL", value: "text-5xl" },
	{ label: "6XL", value: "text-6xl" },
];

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
	fontSize: "text-5xl",
	fontFamily: "Roboto Mono",
	textColor: "#ffffff",
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

<div class="h-screen p-8 overflow-hidden" style="background-color: {theme.bg}; color: {theme.fg};">
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
			<label class="block text-sm mb-2" style="color: {theme.grey};">Font Size</label>
			<div class="grid grid-cols-5 gap-2">
				{#each fontSizes as size}
					<button
						onclick={() => { settings.fontSize = size.value; saveSettings(); }}
						class="px-3 py-2 rounded border"
						style="background-color: {settings.fontSize === size.value ? theme.blue : theme.bg_dark}; border-color: {settings.fontSize === size.value ? theme.blue : theme.border}; color: {theme.fg};"
					>
						{size.label}
					</button>
				{/each}
			</div>
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

		<div class="pt-6" style="border-top: 1px solid {theme.border};">
			<h2 class="text-lg font-semibold mb-4">Preview</h2>
			<div
				class="rounded-xl p-6 flex items-center justify-center bg-black/90"
				style="color: {settings.textColor}; font-family: {settings.fontFamily};"
			>
				<span class="{settings.fontSize}">Abc 123</span>
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
