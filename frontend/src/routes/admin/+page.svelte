<script lang="ts">
	// Administration de l'instance (phase 32). Politique d'inscription, limite
	// d'upload, quotas par compte (lot 32), invitations, création /
	// réinitialisation / suppression de comptes (/api/admin/*). Liste des
	// comptes : GET /admin/users (lot 32, recherche + pages), seule source (la
	// liste locale « comptes créés depuis ce navigateur » de la phase 32 est retirée).
	import { onMount } from 'svelte';
	import { ApiError } from '$lib/api/client';
	import {
		createInvitation,
		createUser,
		deleteUser,
		listUsers,
		patchInstance,
		resetPassword,
		setUserStatus
	} from '$lib/api/admin';
	import { formatBytes } from '$lib/api/quota';
	import type { AdminUserListResponse, AdminUserResponse } from '$lib/api/types';
	import AppSidebar from '$lib/nav/AppSidebar.svelte';
	import {
		POLICIES,
		bytesToMb,
		generatePassword,
		invitationLink,
		mbToBytes,
		pageLabel,
		parseSliceLimit,
		quotaMbToBytes,
		type RegistrationPolicy
	} from '$lib/admin/admin';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const errorText = (e: unknown, fallback: string) =>
		e instanceof ApiError ? e.message : fallback;

	// — Inscriptions et quota d'upload
	let policy = $state<RegistrationPolicy>('open');
	let uploadMb = $state(500);
	let savedPolicy = $state<RegistrationPolicy>('open');
	let savedMb = $state(500);
	let instanceMsg = $state<{ ok: boolean; text: string } | null>(null);
	let savingInstance = $state(false);
	// Quotas par compte (lot 32) : 0 = illimité. Absents d'un serveur antérieur.
	const quotasSupported = $derived(data.instance.storage_quota_bytes !== undefined);
	let storageMb = $state(0);
	let savedStorageMb = $state(0);
	let sliceLimit = $state(0);
	let savedSliceLimit = $state(0);
	onMount(() => {
		policy = savedPolicy = (data.instance.registration_policy as RegistrationPolicy) ?? 'open';
		uploadMb = savedMb = bytesToMb(data.instance.upload_limit_bytes);
		storageMb = savedStorageMb = bytesToMb(data.instance.storage_quota_bytes ?? 0);
		sliceLimit = savedSliceLimit = data.instance.max_concurrent_slices ?? 0;
	});
	const instanceDirty = $derived(
		policy !== savedPolicy ||
			uploadMb !== savedMb ||
			storageMb !== savedStorageMb ||
			sliceLimit !== savedSliceLimit
	);

	async function saveInstance() {
		const bytes = mbToBytes(uploadMb);
		if (bytes === null) {
			instanceMsg = { ok: false, text: "La limite d'upload doit être un nombre de Mo positif." };
			return;
		}
		const storageBytes = quotaMbToBytes(storageMb);
		const slices = parseSliceLimit(sliceLimit);
		if (quotasSupported && storageBytes === null) {
			instanceMsg = { ok: false, text: 'Le quota de stockage doit être un nombre de Mo ≥ 0.' };
			return;
		}
		if (quotasSupported && slices === null) {
			instanceMsg = {
				ok: false,
				text: 'Les tranchages simultanés doivent être un entier entre 0 et 1000.'
			};
			return;
		}
		savingInstance = true;
		instanceMsg = null;
		try {
			const res = await patchInstance({
				registration_policy: policy,
				upload_limit_bytes: BigInt(bytes),
				...(quotasSupported
					? { storage_quota_bytes: storageBytes ?? 0, max_concurrent_slices: slices ?? 0 }
					: {})
			});
			policy = savedPolicy = res.registration_policy as RegistrationPolicy;
			uploadMb = savedMb = bytesToMb(res.upload_limit_bytes);
			storageMb = savedStorageMb = bytesToMb(res.storage_quota_bytes ?? 0);
			sliceLimit = savedSliceLimit = res.max_concurrent_slices ?? 0;
			instanceMsg = { ok: true, text: 'Réglages de l’instance enregistrés.' };
		} catch (e) {
			instanceMsg = { ok: false, text: errorText(e, 'Enregistrement impossible.') };
		} finally {
			savingInstance = false;
		}
	}

	// — Invitations
	let inviteDays = $state(7);
	let invite = $state<{ link: string; expires: string } | null>(null);
	let inviteMsg = $state<string | null>(null);
	let copied = $state(false);
	async function newInvitation() {
		inviteMsg = null;
		copied = false;
		try {
			const res = await createInvitation(Math.max(1, Math.round(inviteDays)));
			invite = {
				link: invitationLink(location.origin, res.token),
				expires: new Date(res.expires_at).toLocaleString()
			};
		} catch (e) {
			inviteMsg = errorText(e, 'Invitation impossible.');
		}
	}
	async function copyInvite() {
		if (!invite) return;
		try {
			await navigator.clipboard.writeText(invite.link);
			copied = true;
		} catch {
			copied = false;
		}
	}

	// — Comptes (GET /admin/users, lot 32). null = pas encore chargée / erreur.
	const PER_PAGE = 25;
	let serverList = $state<AdminUserListResponse | null>(null);
	let listError = $state<string | null>(null);
	let search = $state('');
	let page = $state(1);
	let searchTimer: ReturnType<typeof setTimeout> | undefined;
	let listSeq = 0;
	async function loadUsers() {
		const seq = ++listSeq;
		try {
			const res = await listUsers({ page, per_page: PER_PAGE, q: search });
			if (seq !== listSeq) return;
			serverList = res;
			listError = null;
		} catch (e) {
			if (seq !== listSeq) return;
			serverList = null;
			listError =
				e instanceof ApiError && (e.status === 404 || e.status === 405)
					? 'Ce serveur ne fournit pas la liste des comptes (mise à jour du serveur requise).'
					: errorText(e, 'Liste des comptes indisponible.');
		}
	}
	onMount(() => void loadUsers());
	function onSearch() {
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => {
			page = 1;
			void loadUsers();
		}, 300);
	}
	function goPage(delta: number) {
		page = Math.max(1, page + delta);
		void loadUsers();
	}
	const rows = $derived<AdminUserResponse[]>(serverList?.users ?? []);
	const lastPage = $derived(
		serverList ? Math.max(1, Math.ceil(serverList.total / serverList.per_page)) : 1
	);

	let newEmail = $state('');
	let newPassword = $state('');
	let newRole = $state<'user' | 'admin'>('user');
	let createMsg = $state<{ ok: boolean; text: string } | null>(null);
	let creating = $state(false);
	async function create(event: SubmitEvent) {
		event.preventDefault();
		createMsg = null;
		if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(newEmail.trim())) {
			createMsg = { ok: false, text: 'Adresse e-mail invalide.' };
			return;
		}
		if (newPassword.length < 8) {
			createMsg = { ok: false, text: 'Mot de passe trop court (8 caractères minimum).' };
			return;
		}
		creating = true;
		try {
			const user = await createUser({
				email: newEmail.trim(),
				password: newPassword,
				role: newRole
			});
			void loadUsers();
			createMsg = {
				ok: true,
				text: `Compte ${user.email} créé. Communiquez-lui son mot de passe : ${newPassword}`
			};
			newEmail = '';
			newPassword = '';
			newRole = 'user';
		} catch (e) {
			createMsg = { ok: false, text: errorText(e, 'Création impossible.') };
		} finally {
			creating = false;
		}
	}

	// Réinitialisation / suppression d'un compte de la liste.
	let rowMsg = $state<Record<string, { ok: boolean; text: string }>>({});
	let resetFor = $state<string | null>(null);
	let resetValue = $state('');
	let confirmDelete = $state<string | null>(null);
	async function doReset(user: AdminUserResponse) {
		if (resetValue.length < 8) {
			rowMsg = { ...rowMsg, [user.id]: { ok: false, text: '8 caractères minimum.' } };
			return;
		}
		try {
			await resetPassword(user.id, resetValue);
			rowMsg = {
				...rowMsg,
				[user.id]: { ok: true, text: `Nouveau mot de passe : ${resetValue}` }
			};
			resetFor = null;
			resetValue = '';
		} catch (e) {
			rowMsg = { ...rowMsg, [user.id]: { ok: false, text: errorText(e, 'Échec.') } };
		}
	}
	// Désactivation / réactivation (lot 35, PUT /admin/users/{id}/status) : un
	// compte désactivé est déconnecté aussitôt (sessions → 401) et ne peut plus
	// se connecter ; le dernier administrateur actif est protégé (403).
	async function doToggleStatus(user: AdminUserResponse) {
		try {
			await setUserStatus(user.id, user.status === 'disabled' ? 'active' : 'disabled');
			void loadUsers();
		} catch (e) {
			rowMsg = { ...rowMsg, [user.id]: { ok: false, text: errorText(e, 'Échec.') } };
		}
	}
	async function doDelete(user: AdminUserResponse) {
		try {
			await deleteUser(user.id);
			confirmDelete = null;
			void loadUsers();
		} catch (e) {
			// Compte déjà supprimé ailleurs : la liste rechargée ne l'affiche plus.
			if (e instanceof ApiError && e.status === 404) void loadUsers();
			else rowMsg = { ...rowMsg, [user.id]: { ok: false, text: errorText(e, 'Échec.') } };
			confirmDelete = null;
		}
	}
</script>

<svelte:head><title>Administration — Web-Slicer</title></svelte:head>

<div class="flex h-screen bg-surface text-content">
	<AppSidebar email={data.user.email} active="admin" />
	<div class="flex-1 overflow-auto">
		<main class="mx-auto flex max-w-3xl flex-col gap-8 p-6" data-testid="admin-page">
			<h1 class="ws-page-title">Administration</h1>

			<section class="flex flex-col gap-3" aria-labelledby="admin-instance">
				<h2 id="admin-instance" class="ws-section-title">Inscriptions et quota</h2>
				<fieldset class="flex flex-col gap-2">
					<legend class="mb-1 text-sm text-content-muted">Inscriptions</legend>
					{#each POLICIES as p (p.value)}
						<label class="flex items-start gap-2 text-sm">
							<input
								type="radio"
								name="policy"
								value={p.value}
								bind:group={policy}
								data-testid="admin-policy-{p.value}"
							/>
							<span><span class="font-medium">{p.label}</span> — {p.hint}</span>
						</label>
					{/each}
				</fieldset>
				<label class="flex items-center gap-2 text-sm">
					<span class="text-content-muted">Taille maximale d'un fichier importé</span>
					<input
						type="number"
						min="1"
						step="1"
						class="ws-field ws-field-sm w-24"
						bind:value={uploadMb}
						data-testid="admin-upload-limit"
					/>
					<span>Mo</span>
				</label>
				{#if quotasSupported}
					<label class="flex items-center gap-2 text-sm">
						<span class="text-content-muted">Stockage maximal par compte</span>
						<input
							type="number"
							min="0"
							step="1"
							class="ws-field ws-field-sm w-24"
							bind:value={storageMb}
							data-testid="admin-storage-quota"
						/>
						<span>Mo</span>
						<span class="text-xs text-content-subtle">0 = illimité</span>
					</label>
					<label class="flex items-center gap-2 text-sm">
						<span class="text-content-muted">Tranchages simultanés par compte</span>
						<input
							type="number"
							min="0"
							max="1000"
							step="1"
							class="ws-field ws-field-sm w-20"
							bind:value={sliceLimit}
							data-testid="admin-slice-quota"
						/>
						<span class="text-xs text-content-subtle">0 = illimité</span>
					</label>
				{/if}
				<div class="flex items-center gap-3">
					<button
						type="button"
						class="ws-btn ws-btn-primary"
						disabled={!instanceDirty || savingInstance}
						onclick={saveInstance}
						data-testid="admin-save-instance">Enregistrer</button
					>
					{#if instanceMsg}
						<span
							class="text-sm {instanceMsg.ok ? 'text-success' : 'text-danger'}"
							role={instanceMsg.ok ? 'status' : 'alert'}>{instanceMsg.text}</span
						>
					{/if}
				</div>
				{#if !quotasSupported}
					<p class="text-xs text-content-subtle">
						Ce serveur ne gère pas encore de quota de stockage ni de tranchages par compte : seule
						la taille des fichiers importés est limitée.
					</p>
				{/if}
			</section>

			<section class="flex flex-col gap-3" aria-labelledby="admin-invite">
				<h2 id="admin-invite" class="ws-section-title">Inviter quelqu'un</h2>
				<div class="flex flex-wrap items-center gap-2 text-sm">
					<label class="flex items-center gap-2">
						<span class="text-content-muted">Valable</span>
						<input
							type="number"
							min="1"
							max="365"
							class="ws-field ws-field-sm w-20"
							bind:value={inviteDays}
						/>
						<span>jour(s)</span>
					</label>
					<button
						type="button"
						class="ws-btn ws-btn-secondary"
						onclick={newInvitation}
						data-testid="admin-invite">Créer un lien d'invitation</button
					>
				</div>
				{#if invite}
					<div class="flex flex-col gap-1 rounded border border-border p-3 text-sm">
						<div class="flex items-center gap-2">
							<input
								class="ws-field ws-field-sm min-w-0 flex-1 font-mono text-xs"
								readonly
								value={invite.link}
								aria-label="Lien d'invitation"
								data-testid="admin-invite-link"
							/>
							<button type="button" class="ws-btn ws-btn-sm ws-btn-secondary" onclick={copyInvite}>
								{copied ? 'Copié' : 'Copier'}
							</button>
						</div>
						<span class="text-xs text-content-muted"
							>Usage unique, expire le {invite.expires}. Fonctionne quelle que soit la politique
							d'inscription.</span
						>
					</div>
				{/if}
				{#if inviteMsg}<p class="text-sm text-danger" role="alert">{inviteMsg}</p>{/if}
			</section>

			<section class="flex flex-col gap-3" aria-labelledby="admin-users">
				<h2 id="admin-users" class="ws-section-title">Comptes</h2>
				<form class="flex flex-wrap items-end gap-2 text-sm" onsubmit={create} novalidate>
					<label class="flex flex-col gap-1">
						<span class="text-content-muted">E-mail</span>
						<input
							type="email"
							class="ws-field ws-field-sm w-56"
							bind:value={newEmail}
							autocomplete="off"
							data-testid="admin-user-email"
						/>
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-content-muted">Mot de passe</span>
						<span class="flex gap-1">
							<input
								type="text"
								class="ws-field ws-field-sm w-40 font-mono"
								bind:value={newPassword}
								autocomplete="off"
								data-testid="admin-user-password"
							/>
							<button
								type="button"
								class="ws-btn ws-btn-sm ws-btn-secondary"
								onclick={() => (newPassword = generatePassword())}>Générer</button
							>
						</span>
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-content-muted">Rôle</span>
						<select class="ws-field ws-field-sm" bind:value={newRole} data-testid="admin-user-role">
							<option value="user">Utilisateur</option>
							<option value="admin">Administrateur</option>
						</select>
					</label>
					<button
						type="submit"
						class="ws-btn ws-btn-sm ws-btn-primary"
						disabled={creating}
						data-testid="admin-user-create">Créer le compte</button
					>
				</form>
				{#if createMsg}
					<p
						class="text-sm {createMsg.ok ? 'text-success' : 'text-danger'}"
						role={createMsg.ok ? 'status' : 'alert'}
						data-testid="admin-user-message"
					>
						{createMsg.text}
					</p>
				{/if}

				{#if serverList}
					<div class="flex flex-wrap items-center gap-2 text-sm">
						<input
							type="search"
							class="ws-field ws-field-sm w-64"
							placeholder="Rechercher un e-mail"
							aria-label="Rechercher un compte par e-mail"
							bind:value={search}
							oninput={onSearch}
							data-testid="admin-user-search"
						/>
						<span class="text-xs text-content-muted tabular-nums" data-testid="admin-user-count"
							>{pageLabel(serverList.page, serverList.per_page, serverList.total)}</span
						>
						{#if lastPage > 1}
							<span class="ml-auto flex gap-1">
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-secondary"
									disabled={page <= 1}
									onclick={() => goPage(-1)}>Précédents</button
								>
								<button
									type="button"
									class="ws-btn ws-btn-sm ws-btn-secondary"
									disabled={page >= lastPage}
									onclick={() => goPage(1)}>Suivants</button
								>
							</span>
						{/if}
					</div>
				{/if}
				{#if listError}<p class="text-sm text-danger" role="alert">{listError}</p>{/if}
				{#if !serverList}
					{#if !listError}
						<p class="text-sm text-content-muted">Chargement des comptes…</p>
					{/if}
				{:else if rows.length === 0}
					<p class="ws-empty rounded-lg border border-dashed border-border py-4 text-sm">
						{search.trim() ? 'Aucun compte ne correspond.' : 'Aucun compte.'}
					</p>
				{:else}
					<ul class="flex flex-col divide-y divide-border rounded border border-border">
						{#each rows as user (user.id)}
							<li
								class="flex flex-col gap-1 p-2 text-sm {user.status === 'disabled'
									? 'opacity-60'
									: ''}"
								data-testid="admin-user-row"
							>
								<!-- L'e-mail garde au moins 14rem ; les boutons passent à la ligne
								     plutôt que de réduire l'adresse à « a36mw… ». -->
								<div class="flex flex-wrap items-center gap-x-2 gap-y-1">
									<span class="min-w-[14rem] flex-1 truncate" title={user.email}
										>{user.email}{#if user.id === data.user.id}
											<span class="text-xs text-content-subtle">(vous)</span>{/if}</span
									>
									<span
										class="text-xs text-content-muted tabular-nums"
										title={'Créé le ' + new Date(user.created_at).toLocaleDateString()}
										>{user.project_count} projet{user.project_count === 1 ? '' : 's'} · {formatBytes(
											user.storage_bytes
										)}</span
									>
									<span class="text-xs text-content-muted"
										>{user.role === 'admin' ? 'Administrateur' : 'Utilisateur'}</span
									>
									{#if user.status === 'disabled'}
										<span
											class="rounded bg-danger-bg px-1.5 py-0.5 text-xs text-danger-text"
											data-testid="admin-user-status">Désactivé</span
										>
									{:else}
										<span class="text-xs text-success-text" data-testid="admin-user-status"
											>Actif</span
										>
									{/if}
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-secondary"
										onclick={() => {
											resetFor = resetFor === user.id ? null : user.id;
											resetValue = generatePassword();
										}}>Réinitialiser le mot de passe</button
									>
									{#if user.status === 'disabled'}
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-primary"
											data-testid="admin-user-enable"
											title="Autorise de nouveau la connexion du compte"
											onclick={() => doToggleStatus(user)}>Réactiver</button
										>
									{:else}
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-secondary"
											data-testid="admin-user-disable"
											title="Ferme ses sessions et refuse ses connexions (réversible)"
											disabled={user.id === data.user.id}
											onclick={() => doToggleStatus(user)}>Désactiver</button
										>
									{/if}
									{#if confirmDelete === user.id}
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-danger"
											onclick={() => doDelete(user)}>Confirmer la suppression</button
										>
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-secondary"
											onclick={() => (confirmDelete = null)}>Annuler</button
										>
									{:else}
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-secondary"
											onclick={() => (confirmDelete = user.id)}>Supprimer…</button
										>
									{/if}
								</div>
								{#if confirmDelete === user.id}
									<p class="text-xs text-danger">
										Supprime le compte, ses projets, modèles et G-code. Irréversible.
									</p>
								{/if}
								{#if resetFor === user.id}
									<div class="flex items-center gap-2">
										<input
											type="text"
											class="ws-field ws-field-sm w-44 font-mono"
											bind:value={resetValue}
											aria-label="Nouveau mot de passe de {user.email}"
										/>
										<button
											type="button"
											class="ws-btn ws-btn-sm ws-btn-primary"
											onclick={() => doReset(user)}>Appliquer</button
										>
									</div>
								{/if}
								{#if rowMsg[user.id]}
									<span
										class="text-xs {rowMsg[user.id].ok ? 'text-success' : 'text-danger'}"
										role={rowMsg[user.id].ok ? 'status' : 'alert'}>{rowMsg[user.id].text}</span
									>
								{/if}
							</li>
						{/each}
					</ul>
				{/if}
			</section>
		</main>
	</div>
</div>
