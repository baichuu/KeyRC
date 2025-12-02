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

const keyAliases: Record<string, string> = {
	"Backspace": "⌫",
	"Enter": "↵",
	"Tab": "⇥",
	"Space": "␣",
	"Escape": "Esc",
	"Delete": "⌦",
	"Home": "⇱",
	"End": "⇲",
	"PageUp": "PgUp",
	"PageDown": "PgDn",
	"Insert": "Ins",
	"CapsLock": "⇪",
	"PrintScreen": "PrtSc",
	"ScrollLock": "ScrLk",
	"Pause": "⏸",
	"NumLock": "Num",
};

let keyHistory = $state<StoredKey[]>([]);
let activeModifiers = $state<string[]>([]);

function startDrag() {
	getCurrentWindow().startDragging();
}

function displayKey(stored: StoredKey): string {
	const { key, shift } = stored;
	if (keyAliases[key]) return keyAliases[key];
	if (key.length === 1) {
		return shift ? key.toUpperCase() : key.toLowerCase();
	}
	return key;
}

onMount(() => {
	listen<KeyEvent>("key-event", (event) => {
		const { key, modifiers } = event.payload;

		activeModifiers = modifiers;
		const shift = modifiers.includes("Shift");
		keyHistory = [{ key, shift }, ...keyHistory].slice(0, 4);
	});
});
</script>

<main
  onmousedown={startDrag}
	class="w-full h-screen cursor-move flex flex-col justify-center items-center gap-1"
>
	<div
		class="bg-black/90 backdrop-blur-md flex-1 w-full flex flex-row justify-center items-center rounded-tr-3xl rounded-tl-3xl overflow-hidden"
	>
		<h1 class="text-6xl truncate px-4 py-2">{keyHistory.toReversed().map(displayKey).join("")}</h1>
	</div>
	<div class="h-14 shrink-0 grid grid-cols-4 gap-1 w-full rounded-br-3xl rounded-bl-3xl overflow-hidden">
		<div class="flex items-center justify-center bg-black/90 backdrop-blur-md transition-colors {activeModifiers.includes('Shift') ? 'text-white' : 'text-white/30'}">
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="2em"
				height="2em"
				viewBox="0 0 16 16"
			>
				<title>shift</title>
				<path
					fill="currentColor"
					d="M7.27 2.047a1 1 0 0 1 1.46 0l6.345 6.77c.6.638.146 1.683-.73 1.683H11.5v3a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1v-3H1.654C.78 10.5.326 9.455.924 8.816zM14.346 9.5L8 2.731L1.654 9.5H4.5a1 1 0 0 1 1 1v3h5v-3a1 1 0 0 1 1-1z"
				/>
			</svg>
		</div>
		<div class="flex items-center justify-center bg-black/90 backdrop-blur-md transition-colors {activeModifiers.includes('Ctrl') ? 'text-white' : 'text-white/30'}">
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="2em"
				height="2em"
				viewBox="0 0 16 16"
			>
				<title>ctrl</title>
				<path
					fill="currentColor"
					d="M11.5 7a.5.5 0 0 1-.377-.171l-3.124-3.57l-3.124 3.57a.5.5 0 1 1-.753-.659l3.5-4a.502.502 0 0 1 .752 0l3.5 4a.5.5 0 0 1-.376.83z"
				/>
			</svg>
		</div>
		<div class="flex items-center justify-center bg-black/90 backdrop-blur-md transition-colors {activeModifiers.includes('Alt') ? 'text-white' : 'text-white/30'}">
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="2em"
				height="2em"
				viewBox="0 0 24 24"
			>
				<title>alt</title>
				<path
					fill="none"
					stroke="currentColor"
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="1.5"
					d="M3 5.25h5.625l6.75 13.5H21m-6.75-13.5H21"
				/>
			</svg>
		</div>
		<div class="flex items-center justify-center bg-black/90 backdrop-blur-md transition-colors {activeModifiers.includes('Super') ? 'text-white' : 'text-white/30'}">
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="2em"
				height="2em"
				viewBox="0 0 24 24"
			>
				<title>command</title>
				<path
					fill="none"
					stroke="currentColor"
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="1.5"
					d="M15.012 5.977v12.046c0 2.645 3.316 3.954 5.14 2.13c1.825-1.825.516-5.141-2.13-5.141H5.978c-2.645 0-3.953 3.316-2.13 5.14c1.825 1.825 5.142.516 5.142-2.13V5.978c0-2.645-3.317-3.953-5.141-2.13c-1.824 1.825-.516 5.142 2.13 5.142h12.045c2.645 0 3.954-3.317 2.13-5.141s-5.141-.516-5.141 2.13"
				/>
			</svg>
		</div>
	</div>
</main>
