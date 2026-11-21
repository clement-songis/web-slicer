<script lang="ts">
	// SSAO de la vue réaliste (phase 39) : rendu de la scène dans une cible
	// couleur + profondeur, puis passe plein écran (ssao.ts, shader d'Orca).
	// Monté seulement si la préférence est active : aucun coût sinon. Rendu à
	// la demande conservé (la tâche ne tourne qu'à l'invalidation).
	import { useTask, useThrelte } from '@threlte/core';
	import * as THREE from 'three';
	import { onMount } from 'svelte';
	import { PLATE_TOLERANCE_MM, SSAO_FRAGMENT, SSAO_VERTEX, ssaoPlate } from './ssao';
	import type { BedShape } from './bed';

	interface Props {
		bed: BedShape;
	}
	let { bed }: Props = $props();

	const { renderer, scene, camera, autoRender, renderStage, invalidate } = useThrelte();

	// Cible 8 bits sRGB (décodée en linéaire à la lecture) : moitié moins de
	// bande passante qu'un tampon demi-flottant, sans bandes visibles.
	const target = new THREE.WebGLRenderTarget(1, 1, {
		type: THREE.UnsignedByteType,
		colorSpace: THREE.SRGBColorSpace,
		depthBuffer: true
	});
	target.depthTexture = new THREE.DepthTexture(1, 1);
	target.depthTexture.type = THREE.UnsignedIntType;
	const material = new THREE.ShaderMaterial({
		vertexShader: SSAO_VERTEX,
		fragmentShader: SSAO_FRAGMENT,
		depthTest: false,
		depthWrite: false,
		uniforms: {
			tColor: { value: target.texture },
			tDepth: { value: target.depthTexture },
			invTexSize: { value: new THREE.Vector2(1, 1) },
			zNear: { value: 0.1 },
			zFar: { value: 1000 },
			ortho: { value: false },
			projectionInverse: { value: new THREE.Matrix4() },
			viewInverse: { value: new THREE.Matrix4() },
			plate: { value: new THREE.Vector4() },
			plateCircle: { value: new THREE.Vector3() },
			plateTolerance: { value: PLATE_TOLERANCE_MM }
		}
	});
	const quad = new THREE.Mesh(new THREE.PlaneGeometry(2, 2), material);
	quad.frustumCulled = false;
	const quadScene = new THREE.Scene();
	quadScene.add(quad);
	const quadCamera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
	const size = new THREE.Vector2();
	// Le premier rendu (vers la cible) ne compte pas comme une image (compteur
	// d'i/s) : rendu d'origine exposé par rendererFactory.
	const renderToTarget =
		(renderer as THREE.WebGLRenderer & { renderUncounted?: THREE.WebGLRenderer['render'] })
			.renderUncounted ?? renderer.render.bind(renderer);

	// MSAA de la cible = celui du contexte (préférence « Anticrénelage »).
	target.samples = renderer.getContext().getContextAttributes()?.antialias ? 4 : 0;
	$effect(() => {
		const p = ssaoPlate(bed);
		material.uniforms.plate.value.set(...p.rect);
		material.uniforms.plateCircle.value.set(...p.circle);
		invalidate();
	});

	onMount(() => {
		const previous = autoRender.current;
		autoRender.set(false);
		invalidate();
		return () => {
			autoRender.set(previous);
			target.dispose();
			material.dispose();
			quad.geometry.dispose();
			invalidate();
		};
	});

	useTask(
		() => {
			const cam = camera.current as THREE.PerspectiveCamera | THREE.OrthographicCamera;
			renderer.getDrawingBufferSize(size);
			if (target.width !== size.x || target.height !== size.y) {
				target.setSize(size.x, size.y);
				target.depthTexture!.image.width = size.x;
				target.depthTexture!.image.height = size.y;
			}
			renderer.setRenderTarget(target);
			renderToTarget(scene, cam);
			renderer.setRenderTarget(null);
			const u = material.uniforms;
			u.invTexSize.value.set(1 / size.x, 1 / size.y);
			u.zNear.value = cam.near;
			u.zFar.value = cam.far;
			u.ortho.value = (cam as THREE.OrthographicCamera).isOrthographicCamera === true;
			u.projectionInverse.value.copy(cam.projectionMatrixInverse);
			u.viewInverse.value.copy(cam.matrixWorld);
			renderer.render(quadScene, quadCamera);
		},
		{ stage: renderStage, autoInvalidate: false }
	);
</script>
