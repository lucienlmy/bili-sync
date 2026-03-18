<script lang="ts">
	import api from '$lib/api';
	import { setBreadcrumb } from '$lib/stores/breadcrumb';
	import { onMount, tick } from 'svelte';
	import { Badge } from '$lib/components/ui/badge';

	let unsubscribeLog: (() => void) | null = null;
	let logs: Array<{ timestamp: string; level: string; message: string }> = [];
	let progressMap: Record<string, { done: number; total: number; percent: number; updatedAt: number }> = {};
	let shouldAutoScroll = true;
	let main: HTMLElement | null = null;
	let scrollTimer: ReturnType<typeof setTimeout> | null = null;

	function parseLayer(message: string): { layer: 'video' | 'page' | 'file' | null; text: string } {
		if (message.startsWith('【视频层】')) {
			return { layer: 'video', text: message.replace('【视频层】', '').trim() };
		}
		if (message.startsWith('【分页层】')) {
			return { layer: 'page', text: message.replace('【分页层】', '').trim() };
		}
		if (message.startsWith('【文件层】')) {
			return { layer: 'file', text: message.replace('【文件层】', '').trim() };
		}
		return { layer: null, text: message };
	}

	function getLayerClass(layer: 'video' | 'page' | 'file' | null) {
		switch (layer) {
			case 'video':
				return 'bg-blue-50 text-blue-700 border border-blue-100';
			case 'page':
				return 'bg-indigo-50 text-indigo-700 border border-indigo-100';
			case 'file':
				return 'bg-emerald-50 text-emerald-700 border border-emerald-100';
			default:
				return 'bg-transparent text-muted-foreground';
		}
	}

	function checkScrollPosition() {
		if (main) {
			const { scrollTop, scrollHeight, clientHeight } = main;
			shouldAutoScroll = scrollTop + clientHeight >= scrollHeight - 50;
		}
	}

	async function scrollToBottom() {
		await tick();
		if (shouldAutoScroll && main) {
			main.scrollTop = main.scrollHeight;
		}
	}

	onMount(() => {
		setBreadcrumb([{ label: '日志' }]);
		main = document.getElementById('main');
		main?.addEventListener('scroll', checkScrollPosition);
		unsubscribeLog = api.subscribeToLogs((data: string) => {
			const entry = JSON.parse(data);
			const msg: string = entry.message || '';
			if (msg.startsWith('【文件层进度】')) {
				try {
					const json = JSON.parse(msg.replace('【文件层进度】', ''));
					const id = json.id || json.path || 'unknown';
					progressMap[id] = { done: json.done || 0, total: json.total || 0, percent: json.percent || 0, updatedAt: Date.now() };
					// keep a small recent logs buffer too
					logs = [...logs.slice(-499), entry];
				} catch (e) {
					logs = [...logs.slice(-499), entry];
				}
			} else {
				logs = [...logs.slice(-499), entry];
			}
			if (scrollTimer) clearTimeout(scrollTimer);
			scrollTimer = setTimeout(scrollToBottom, 20);
		});
		return () => {
			if (scrollTimer) clearTimeout(scrollTimer);
			main?.removeEventListener('scroll', checkScrollPosition);
			if (unsubscribeLog) {
				unsubscribeLog();
				unsubscribeLog = null;
			}
		};
	});

	function getLevelColor(level: string) {
		switch (level) {
			case 'ERROR':
				return 'text-rose-600';
			case 'WARN':
				return 'text-yellow-600';
			case 'INFO':
			default:
				return 'text-emerald-600';
		}
	}
</script>

<svelte:head>
	<title>日志 - Bili Sync</title>
</svelte:head>

<div class="space-y-1">
	{#each logs as log, index (index)}
			{#key index}
				{@const parsed = parseLayer(log.message)}
			<div
				class="flex items-center gap-3 rounded-md p-1 font-mono text-xs {index % 2 === 0
					? 'bg-muted/50'
					: 'bg-background'}"
			>
				<span class="text-muted-foreground w-32 shrink-0">
					{log.timestamp}
				</span>
				<Badge
					class="w-16 shrink-0 justify-center {getLevelColor(log.level)} bg-primary/90 font-semibold"
				>
					{log.level}
				</Badge>
				{#if parsed.layer}
					<span class="px-2 py-0.5 rounded-md text-xs font-medium mr-2 {getLayerClass(parsed.layer)}">
						{parsed.layer === 'video' ? '视频层' : parsed.layer === 'page' ? '分页层' : '文件层'}
					</span>
				{/if}
				<span class="flex-1 break-all">{parsed.text}</span>
			</div>
		{/key}
	{/each}
		{#if Object.keys(progressMap).length > 0}
			<div class="space-y-2 mb-2">
				{#each Object.entries(progressMap) as [id, p]}
					<div class="flex items-center gap-3 font-mono text-xs">
						<div class="w-32 text-muted-foreground truncate">{id}</div>
						<div class="flex-1">
							<div class="w-full bg-gray-200 rounded h-3 overflow-hidden">
								<div class="h-3 bg-emerald-500" style="width: {Math.min(100, p.percent)}%"></div>
							</div>
							<div class="text-xs text-muted-foreground">{p.percent}% — {Math.round(p.done/1024)} KB / {p.total ? Math.round(p.total/1024) + ' KB' : '??'}</div>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	{#if logs.length === 0}
		<div class="text-muted-foreground py-8 text-center">暂无日志记录</div>
	{/if}
</div>
