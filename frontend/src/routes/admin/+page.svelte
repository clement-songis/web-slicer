<script lang="ts">
	// Administration de l'instance (phase 32). Le serveur expose déjà politique
	// d'inscription, limite d'upload, création / réinitialisation / suppression
	// de comptes et invitations (/api/admin/*) ; aucune interface n'y donnait
	// accès. Pas de liste des comptes côté serveur : les comptes créés ici sont
	// mémorisés dans ce navigateur pour pouvoir les gérer ensuite.
	import { onMount } from 'svelte';
	import { ApiError } from '$lib/api/client';
	import {
		createInvitation,
		createUser,
		deleteUser,
		patchInstance,
		resetPassword
	} from '$lib/api/admin';
	import AppSidebar from '$lib/nav/AppSidebar.svelte';
	import {
		POLICIES,
		bytesToMb,
		generatePassword,
		invitationLink,
		mbToBytes,
		parseManaged,
		upsertManaged,
		type ManagedUser,
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
	onMount(() => {
		policy = savedPolicy = (data.instance.registration_policy as RegistrationPolicy) ?? 'open';
		uploadMb = savedMb = bytesToMb(data.instance.upload_limit_bytes);
	});
	const instanceDirty = $derived(policy !== savedPolicy || uploadMb !== savedMb);

	async function saveInstance() {
		const bytes = mbToBytes(uploadMb);
		if (bytes === null) {
			instanceMsg = { ok: false, text: "La limite d'upload doit être un nombre de Mo positif." };
			return;
		}
		savingInstance = true;
		instanceMsg = null;
		try {
			const res = await patchInstance({
				registration_policy: policy,
				upload_limit_bytes: BigInt(bytes)
			});
			policy = savedPolicy = res.registration_policy as RegistrationPolicy;
			uploadMb = savedMb = bytesToMb(res.upload_limit_bytes);
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

	// — Comptes gérés
	const STORE_KEY = $derived(`web-slicer.admin.managed.${data.user.id}`);
	let managed = $state<ManagedUser[]>([]);
	onMount(() => {
		managed = parseManaged(localStorage.getItem(STORE_KEY));
	});
	function persist(list: ManagedUser[]) {
		managed = list;
		localStorage.setItem(STORE_KEY, JSON.stringify(list));
	}

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
			persist(upsertManaged(managed, { id: user.id, email: user.email, role: user.role }));
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

	// Réinitialisation / suppression d'un compte géré (ou saisi par identifiant).
	let rowMsg = $state<Record<string, { ok: boolean; text: string }>>({});
	let resetFor = $state<string | null>(null);
	let resetValue = $state('');
	let confirmDelete = $state<string | null>(null);
	async function doReset(user: ManagedUser) {
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
	async function doDelete(user: ManagedUser) {
		try {
			await deleteUser(user.id);
			persist(managed.filter((u) => u.id !== user.id));
			confirmDelete = null;
		} catch (e) {
			// Compte déjà supprimé ailleurs : on l'oublie aussi.
			if (e instanceof ApiError && e.status === 404)
				persist(managed.filter((u) => u.id !== user.id));
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
				<p class="text-xs text-content-subtle">
					Pas de quota de stockage ni de tranchages par compte pour l'instant : seule la taille des
					fichiers importés est limitée.
				</p>
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

				{#if managed.length === 0}
					<p class="ws-empty rounded-lg border border-dashed border-border py-4 text-sm">
						Aucun compte créé depuis cette page.
					</p>
				{:else}
					<ul class="flex flex-col divide-y divide-border rounded border border-border">
						{#each managed as user (user.id)}
							<li class="flex flex-col gap-1 p-2 text-sm" data-testid="admin-user-row">
								<div class="flex items-center gap-2">
									<span class="min-w-0 flex-1 truncate" title={user.email}>{user.email}</span>
									<span class="text-xs text-content-muted"
										>{user.role === 'admin' ? 'Administrateur' : 'Utilisateur'}</span
									>
									<button
										type="button"
										class="ws-btn ws-btn-sm ws-btn-secondary"
										onclick={() => {
											resetFor = resetFor === user.id ? null : user.id;
											resetValue = generatePassword();
										}}>Réinitialiser le mot de passe</button
									>
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
				<p class="text-xs text-content-subtle">
					Le serveur ne fournit pas encore la liste de tous les comptes : seuls ceux créés depuis ce
					navigateur sont listés ici.
				</p>
			</section>
		</main>
	</div>
</div>
