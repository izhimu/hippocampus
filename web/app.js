// Hippocampus Gateway — Three.js 3D Brain Visualization (Redesigned)
(function () {
    'use strict';

    // --- Brain Region Config ---
    const REGIONS = {
        amygdala: {
            color: 0xff4444, emissive: 0x661111,
            position: [0, -0.45, 0.35], radius: 0.35,
            weight: 0.35, label: '杏仁核',
            // position on brain surface for marker
            surfaceAngle: { theta: Math.PI * 0.7, phi: Math.PI * 0.35 },
        },
        hippocampus: {
            color: 0xffd700, emissive: 0x665500,
            position: [0, 0.05, 0.9], radius: 0.4,
            weight: 0.30, label: '海马体',
            surfaceAngle: { theta: Math.PI * 0.45, phi: Math.PI * 0.15 },
        },
        prefrontal: {
            color: 0x4488ff, emissive: 0x112266,
            position: [0, 0.6, 1.4], radius: 0.38,
            weight: 0.20, label: '前额叶',
            surfaceAngle: { theta: Math.PI * 0.3, phi: 0 },
        },
        temporal: {
            color: 0x44ff88, emissive: 0x116633,
            position: [-1.35, -0.1, 0.3], radius: 0.35,
            weight: 0.15, label: '颞叶',
            surfaceAngle: { theta: Math.PI * 0.55, phi: Math.PI * 0.8 },
        },
    };

    const REGION_KEYS = ['amygdala', 'hippocampus', 'prefrontal', 'temporal'];

    // --- Three.js Setup ---
    let scene, camera, renderer, clock;
    let brainGroup, brainHemisphereL, brainHemisphereR;
    let regionMarkers = {};
    let regionGlows = {};
    let neuralParticles, neuralLines;
    let starField;
    const NEURAL_COUNT = 600;
    const STAR_COUNT = 2000;

    // State
    let animationState = 'idle';
    let activationQueue = [];
    let activationTimer = 0;
    let breathPhase = 0;
    let scores = { amygdala: 0, hippocampus: 0, prefrontal: 0, temporal: 0 };
    let rejectFlash = 0;
    let writeConverge = 0;
    let burstActive = 0;

    // Neural particle data
    let nPositions, nColors, nSizes, nVelocities, nBasePositions;

    function init() {
        scene = new THREE.Scene();
        scene.background = new THREE.Color(0x020208);

        camera = new THREE.PerspectiveCamera(50, 1, 0.1, 200);
        camera.position.set(0, 0.8, 4.2);
        camera.lookAt(0, 0.2, 0);

        renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
        renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
        renderer.toneMapping = THREE.ACESFilmicToneMapping;
        renderer.toneMappingExposure = 1.2;
        const container = document.getElementById('canvas-container');
        renderer.setSize(container.clientWidth, container.clientHeight);
        container.appendChild(renderer.domElement);

        clock = new THREE.Clock();

        createStarField();
        createBrain();
        createRegionMarkers();
        createNeuralNetwork();
        setupLighting();

        window.addEventListener('resize', onResize);
        onResize();

        animate();
        fetchStats();
        fetchBrainStatus();
        loadEngrams('L1');
        connectWS();
    }

    function onResize() {
        const container = document.getElementById('canvas-container');
        const w = container.clientWidth, h = container.clientHeight;
        camera.aspect = w / h;
        camera.updateProjectionMatrix();
        renderer.setSize(w, h);
    }

    // --- Star Field ---
    function createStarField() {
        const positions = new Float32Array(STAR_COUNT * 3);
        const colors = new Float32Array(STAR_COUNT * 3);
        for (let i = 0; i < STAR_COUNT; i++) {
            const r = 30 + Math.random() * 70;
            const theta = Math.random() * Math.PI * 2;
            const phi = Math.acos(2 * Math.random() - 1);
            positions[i * 3] = r * Math.sin(phi) * Math.cos(theta);
            positions[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta);
            positions[i * 3 + 2] = r * Math.cos(phi);
            const bright = 0.3 + Math.random() * 0.7;
            const tint = Math.random();
            colors[i * 3] = bright * (tint > 0.8 ? 1 : 0.8);
            colors[i * 3 + 1] = bright * 0.85;
            colors[i * 3 + 2] = bright * (tint < 0.3 ? 1 : 0.9);
        }
        const geo = new THREE.BufferGeometry();
        geo.setAttribute('position', new THREE.BufferAttribute(positions, 3));
        geo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
        const mat = new THREE.PointsMaterial({
            size: 0.15, vertexColors: true,
            transparent: true, opacity: 0.9,
            blending: THREE.AdditiveBlending, depthWrite: false,
        });
        starField = new THREE.Points(geo, mat);
        scene.add(starField);
    }

    // --- Brain Shape (displaced ellipsoid) ---
    function brainDisplacement(x, y, z) {
        // Base ellipsoid: wider left-right, taller front-back
        const nx = x / 1.45, ny = y / 1.25, nz = z / 1.3;
        let d = Math.sqrt(nx * nx + ny * ny + nz * nz) - 1.0;

        // Squish bottom slightly
        if (y < -0.2) d += (y + 0.2) * -0.15;

        // Central fissure (longitudinal) - slight indent along top middle
        const fissure = Math.exp(-x * x / 0.01) * Math.max(0, y - 0.1) * 0.15;
        d -= fissure;

        // Lateral sulcus (side groove)
        const lateralY = -0.15;
        const lateralZ = 0.3;
        const lateralDist = Math.sqrt((y - lateralY) * (y - lateralY) + (z - lateralZ) * (z - lateralZ));
        const lateral = Math.exp(-lateralDist * lateralDist / 0.08) * 0.12;
        d -= lateral * (1 - Math.abs(x) / 1.5);

        // Frontal lobe bump
        const frontalBump = Math.exp(-((z - 1.2) * (z - 1.2) + (y - 0.4) * (y - 0.4)) / 0.5) * 0.15;
        d -= frontalBump;

        // Temporal lobe bump
        const temporalBump = Math.exp(-((x * x) + (y + 0.3) * (y + 0.3)) / 0.6) * 0.08 * (1 - Math.max(0, z - 0.8) / 1.0);
        d -= temporalBump;

        // Gyri wrinkles (surface noise)
        const wrinkle = Math.sin(x * 12 + y * 8) * Math.cos(y * 10 + z * 6) * 0.02;
        d += wrinkle;

        // Cerebellum bump (back-bottom)
        const cerebBump = Math.exp(-((z + 1.0) * (z + 1.0) + (y + 0.5) * (y + 0.5) + x * x * 0.3) / 0.3) * 0.2;
        d -= cerebBump;

        return d;
    }

    function createBrain() {
        brainGroup = new THREE.Group();
        scene.add(brainGroup);

        // Left hemisphere
        brainHemisphereL = createHemisphere(1);
        brainHemisphereL.position.x = -0.08;
        brainGroup.add(brainHemisphereL);

        // Right hemisphere
        brainHemisphereR = createHemisphere(-1);
        brainHemisphereR.position.x = 0.08;
        brainGroup.add(brainHemisphereR);

        // Inner glow core
        const coreGeo = new THREE.SphereGeometry(0.5, 24, 24);
        const coreMat = new THREE.MeshBasicMaterial({
            color: 0x2233aa,
            transparent: true,
            opacity: 0.06,
            blending: THREE.AdditiveBlending,
            depthWrite: false,
        });
        const core = new THREE.Mesh(coreGeo, coreMat);
        brainGroup.add(core);
    }

    function createHemisphere(side) {
        // Use IcosahedronGeometry for organic look, then displace vertices
        const geo = new THREE.IcosahedronGeometry(1.4, 4);
        const pos = geo.attributes.position;

        for (let i = 0; i < pos.count; i++) {
            let x = pos.getX(i) * side;
            let y = pos.getY(i);
            let z = pos.getZ(i);

            // Only keep this hemisphere side
            if (x * side < -0.05) {
                x *= 0.7;
            }

            const d = brainDisplacement(x, y, z);
            const scale = 1.0 + d * 0.5;
            pos.setXYZ(i, x * scale, y * scale, z * scale);
        }

        geo.computeVertexNormals();

        // Semi-transparent outer shell
        const mat = new THREE.MeshPhysicalMaterial({
            color: 0x887799,
            emissive: 0x110822,
            transparent: true,
            opacity: 0.18,
            roughness: 0.7,
            metalness: 0.1,
            clearcoat: 0.3,
            clearcoatRoughness: 0.5,
            side: THREE.DoubleSide,
            depthWrite: false,
        });

        const mesh = new THREE.Mesh(geo, mat);
        mesh.userData.baseOpacity = 0.18;
        return mesh;
    }

    // --- Region Markers on brain surface ---
    function createRegionMarkers() {
        for (const key of REGION_KEYS) {
            const r = REGIONS[key];

            // Main marker sphere
            const geo = new THREE.SphereGeometry(r.radius * 0.8, 24, 24);
            const mat = new THREE.MeshPhysicalMaterial({
                color: r.color,
                emissive: r.emissive,
                emissiveIntensity: 0.5,
                transparent: true,
                opacity: 0.55,
                roughness: 0.3,
                metalness: 0.2,
                clearcoat: 0.6,
                depthWrite: false,
            });
            const mesh = new THREE.Mesh(geo, mat);
            mesh.position.set(r.position[0], r.position[1], r.position[2]);
            mesh.userData = { key, baseOpacity: 0.55, baseScale: 1 };
            brainGroup.add(mesh);
            regionMarkers[key] = mesh;

            // Outer glow
            const glowGeo = new THREE.SphereGeometry(r.radius * 1.8, 20, 20);
            const glowMat = new THREE.MeshBasicMaterial({
                color: r.color,
                transparent: true,
                opacity: 0.06,
                blending: THREE.AdditiveBlending,
                depthWrite: false,
            });
            const glow = new THREE.Mesh(glowGeo, glowMat);
            glow.position.copy(mesh.position);
            glow.userData = { baseOpacity: 0.06 };
            brainGroup.add(glow);
            regionGlows[key] = glow;
        }
    }

    // --- Neural Network (particles + connection lines) ---
    function createNeuralNetwork() {
        nPositions = new Float32Array(NEURAL_COUNT * 3);
        nColors = new Float32Array(NEURAL_COUNT * 3);
        nSizes = new Float32Array(NEURAL_COUNT);
        nBasePositions = [];
        nVelocities = [];

        for (let i = 0; i < NEURAL_COUNT; i++) {
            // Distribute within brain volume
            const angle = Math.random() * Math.PI * 2;
            const r = Math.pow(Math.random(), 0.5) * 1.3;
            const y = (Math.random() - 0.5) * 2.2;
            const x = Math.cos(angle) * r * (Math.random() > 0.5 ? 1 : -0.9);
            const z = Math.sin(angle) * r;

            nPositions[i * 3] = x;
            nPositions[i * 3 + 1] = y;
            nPositions[i * 3 + 2] = z;
            nBasePositions.push({ x, y, z });

            nColors[i * 3] = 0.3 + Math.random() * 0.2;
            nColors[i * 3 + 1] = 0.4 + Math.random() * 0.2;
            nColors[i * 3 + 2] = 0.8 + Math.random() * 0.2;

            nSizes[i] = 0.02 + Math.random() * 0.03;

            nVelocities.push({
                x: (Math.random() - 0.5) * 0.001,
                y: (Math.random() - 0.5) * 0.001,
                z: (Math.random() - 0.5) * 0.001,
                phase: Math.random() * Math.PI * 2,
                freq: 0.5 + Math.random() * 1.5,
            });
        }

        const geo = new THREE.BufferGeometry();
        geo.setAttribute('position', new THREE.BufferAttribute(nPositions, 3));
        geo.setAttribute('color', new THREE.BufferAttribute(nColors, 3));
        geo.setAttribute('size', new THREE.BufferAttribute(nSizes, 1));

        const mat = new THREE.PointsMaterial({
            size: 0.035,
            vertexColors: true,
            transparent: true,
            opacity: 0.7,
            blending: THREE.AdditiveBlending,
            depthWrite: false,
            sizeAttenuation: true,
        });

        neuralParticles = new THREE.Points(geo, mat);
        brainGroup.add(neuralParticles);

        // Connection lines between nearby particles
        createNeuralLines();
    }

    function createNeuralLines() {
        const linePositions = [];
        const lineColors = [];
        const maxDist = 0.6;
        const maxConnections = 200;

        let connections = 0;
        for (let i = 0; i < NEURAL_COUNT && connections < maxConnections; i += 3) {
            for (let j = i + 1; j < NEURAL_COUNT && connections < maxConnections; j += 3) {
                const dx = nPositions[i * 3] - nPositions[j * 3];
                const dy = nPositions[i * 3 + 1] - nPositions[j * 3 + 1];
                const dz = nPositions[i * 3 + 2] - nPositions[j * 3 + 2];
                const dist = Math.sqrt(dx * dx + dy * dy + dz * dz);
                if (dist < maxDist && Math.random() > 0.5) {
                    linePositions.push(
                        nPositions[i * 3], nPositions[i * 3 + 1], nPositions[i * 3 + 2],
                        nPositions[j * 3], nPositions[j * 3 + 1], nPositions[j * 3 + 2]
                    );
                    const alpha = 1 - dist / maxDist;
                    lineColors.push(0.2 * alpha, 0.3 * alpha, 0.7 * alpha, 0.2 * alpha, 0.3 * alpha, 0.7 * alpha);
                    connections++;
                }
            }
        }

        const geo = new THREE.BufferGeometry();
        geo.setAttribute('position', new THREE.Float32BufferAttribute(linePositions, 3));
        geo.setAttribute('color', new THREE.Float32BufferAttribute(lineColors, 3));

        const mat = new THREE.LineBasicMaterial({
            vertexColors: true,
            transparent: true,
            opacity: 0.12,
            blending: THREE.AdditiveBlending,
            depthWrite: false,
        });

        neuralLines = new THREE.LineSegments(geo, mat);
        brainGroup.add(neuralLines);
    }

    // --- Lighting ---
    function setupLighting() {
        scene.add(new THREE.AmbientLight(0x1a1a3e, 0.6));

        const dir = new THREE.DirectionalLight(0xaabbff, 0.4);
        dir.position.set(3, 5, 4);
        scene.add(dir);

        const rim = new THREE.PointLight(0x6644cc, 0.5, 8);
        rim.position.set(-3, 1, -3);
        scene.add(rim);

        const top = new THREE.PointLight(0x4488ff, 0.3, 6);
        top.position.set(0, 3, 1);
        scene.add(top);

        // Region lights
        for (const key of REGION_KEYS) {
            const r = REGIONS[key];
            const light = new THREE.PointLight(r.color, 0.2, 2.5);
            light.position.set(r.position[0], r.position[1], r.position[2]);
            brainGroup.add(light);
        }
    }

    // --- Animation Loop ---
    function animate() {
        requestAnimationFrame(animate);
        const dt = clock.getDelta();
        const elapsed = clock.elapsedTime;
        breathPhase = elapsed;

        // Brain slow rotation
        if (brainGroup) {
            brainGroup.rotation.y = elapsed * 0.12;
            brainGroup.rotation.x = Math.sin(elapsed * 0.08) * 0.05;
        }

        // Star field subtle rotation
        if (starField) {
            starField.rotation.y = elapsed * 0.005;
            starField.rotation.x = elapsed * 0.002;
        }

        // Region breathing
        for (const key of REGION_KEYS) {
            const mesh = regionMarkers[key];
            const glow = regionGlows[key];
            const score = scores[key] || 0;
            const breath = 1 + Math.sin(breathPhase * 1.0 + REGIONS[key].position[0] * 2) * 0.04;
            const targetScale = mesh.userData.baseScale * breath;

            mesh.scale.lerp(new THREE.Vector3(targetScale, targetScale, targetScale), 0.08);
            glow.scale.lerp(new THREE.Vector3(targetScale * 1.5, targetScale * 1.5, targetScale * 1.5), 0.08);

            // Emissive intensity
            mesh.material.emissiveIntensity = 0.5 + score * 1.5 + Math.sin(breathPhase * 2 + key.length) * 0.1;
            glow.material.opacity = 0.04 + score * 0.12;
        }

        // Brain shell opacity pulse
        if (brainHemisphereL) {
            const op = brainHemisphereL.userData.baseOpacity + Math.sin(breathPhase * 0.5) * 0.02;
            brainHemisphereL.material.opacity = op;
            brainHemisphereR.material.opacity = op;
        }

        // Reject flash
        if (rejectFlash > 0) {
            rejectFlash -= dt * 3;
            if (brainHemisphereL) {
                brainHemisphereL.material.emissive.setHex(0x330000);
                brainHemisphereL.material.emissiveIntensity = Math.max(0, rejectFlash);
                brainHemisphereR.material.emissive.setHex(0x330000);
                brainHemisphereR.material.emissiveIntensity = Math.max(0, rejectFlash);
            }
            // Flash all markers red
            for (const key of REGION_KEYS) {
                regionMarkers[key].material.emissive.setHex(0x440000);
                regionMarkers[key].material.emissiveIntensity = Math.max(0.5, rejectFlash);
            }
        } else {
            if (brainHemisphereL) {
                brainHemisphereL.material.emissive.setHex(0x110822);
                brainHemisphereL.material.emissiveIntensity = 0;
                brainHemisphereR.material.emissive.setHex(0x110822);
                brainHemisphereR.material.emissiveIntensity = 0;
            }
            // Reset marker emissive
            for (const key of REGION_KEYS) {
                regionMarkers[key].material.emissive.setHex(REGIONS[key].emissive);
            }
        }

        // Neural particle animation
        animateNeuralParticles(dt, elapsed);

        // Camera orbit
        camera.position.x = Math.sin(elapsed * 0.1) * 0.4;
        camera.position.y = 0.8 + Math.sin(elapsed * 0.07) * 0.15;
        camera.position.z = 4.2 + Math.cos(elapsed * 0.08) * 0.2;
        camera.lookAt(0, 0.2, 0);

        // Activation queue
        if (activationQueue.length > 0) {
            activationTimer -= dt;
            if (activationTimer <= 0) {
                const next = activationQueue.shift();
                activateRegion(next.key, next.score);
                activationTimer = 0.3;
            }
        }

        // Write converge decay
        if (writeConverge > 0) {
            writeConverge -= dt * 0.8;
        }

        // Burst decay
        if (burstActive > 0) {
            burstActive -= dt * 2;
        }

        renderer.render(scene, camera);
    }

    function animateNeuralParticles(dt, elapsed) {
        const pos = nPositions;
        const col = nColors;
        const hippoPos = REGIONS.hippocampus.position;

        for (let i = 0; i < NEURAL_COUNT; i++) {
            const v = nVelocities[i];
            const base = nBasePositions[i];

            // Floating drift
            pos[i * 3] = base.x + Math.sin(elapsed * v.freq + v.phase) * 0.08;
            pos[i * 3 + 1] = base.y + Math.cos(elapsed * v.freq * 0.7 + v.phase) * 0.06;
            pos[i * 3 + 2] = base.z + Math.sin(elapsed * v.freq * 0.9 + v.phase * 1.3) * 0.07;

            // Write converge: pull toward hippocampus
            if (writeConverge > 0) {
                const t = writeConverge * 0.5;
                pos[i * 3] += (hippoPos[0] - pos[i * 3]) * t * dt;
                pos[i * 3 + 1] += (hippoPos[1] - pos[i * 3 + 1]) * t * dt;
                pos[i * 3 + 2] += (hippoPos[2] - pos[i * 3 + 2]) * t * dt;
                // Golden color during convergence
                col[i * 3] = 0.9 + Math.sin(elapsed * 5 + i) * 0.1;
                col[i * 3 + 1] = 0.7 + Math.sin(elapsed * 5 + i) * 0.1;
                col[i * 3 + 2] = 0.1;
            }
            // Burst: push outward
            else if (burstActive > 0) {
                const dist = Math.sqrt(pos[i * 3] ** 2 + pos[i * 3 + 1] ** 2 + pos[i * 3 + 2] ** 2);
                const norm = Math.max(0.01, dist);
                const push = burstActive * 0.5;
                pos[i * 3] += (pos[i * 3] / norm) * push * dt;
                pos[i * 3 + 1] += (pos[i * 3 + 1] / norm) * push * dt;
                pos[i * 3 + 2] += (pos[i * 3 + 2] / norm) * push * dt;
                // Bright cyan burst color
                col[i * 3] = 0.5 + Math.random() * 0.5;
                col[i * 3 + 1] = 0.8 + Math.random() * 0.2;
                col[i * 3 + 2] = 1.0;
            }
            // Normal: subtle color variation
            else {
                const flicker = 0.3 + Math.sin(elapsed * v.freq * 2 + v.phase) * 0.15;
                col[i * 3] = flicker * 0.6;
                col[i * 3 + 1] = flicker * 0.8;
                col[i * 3 + 2] = 0.6 + flicker * 0.4;
            }
        }

        neuralParticles.geometry.attributes.position.needsUpdate = true;
        neuralParticles.geometry.attributes.color.needsUpdate = true;

        // Pulse neural line opacity
        neuralLines.material.opacity = 0.08 + Math.sin(elapsed * 1.5) * 0.04 + burstActive * 0.15;
    }

    // --- Region Activation ---
    function activateRegion(key, score) {
        const mesh = regionMarkers[key];
        const glow = regionGlows[key];

        mesh.userData.baseScale = 1 + score * 0.6;
        glow.material.opacity = 0.25 + score * 0.3;

        // High score burst
        if (score > 0.5) {
            mesh.userData.baseScale = 1 + score * 1.0;
            burstActive = 1.0;
        }

        setTimeout(() => {
            mesh.userData.baseScale = 1;
        }, 2500);
    }

    function showRejected() {
        rejectFlash = 1.0;
    }

    function showWriting() {
        writeConverge = 1.5;
    }

    function triggerGateAnimation(components, shouldRemember) {
        activationQueue = [];
        activationTimer = 0;

        for (let i = 0; i < REGION_KEYS.length; i++) {
            const key = REGION_KEYS[i];
            activationQueue.push({ key, score: components[key] || 0 });
        }

        if (!shouldRemember) {
            setTimeout(() => showRejected(), 1200);
        } else {
            setTimeout(() => showWriting(), 1200);
        }
    }

    // --- API Calls (unchanged) ---
    async function fetchStats() {
        try {
            const res = await fetch('/api/stats');
            const data = await res.json();
            if (data.status === 'ok') {
                const by = data.by_layer || {};
                document.getElementById('stat-l1').textContent = by.L1 || 0;
                document.getElementById('stat-l2').textContent = by.L2 || 0;
                document.getElementById('stat-l3').textContent = by.L3 || 0;
                document.getElementById('stat-total').textContent = data.total_engrams || 0;
            }
        } catch (e) { console.error('fetchStats error:', e); }
    }

    async function fetchBrainStatus() {
        try {
            const res = await fetch('/api/brain/status');
            const data = await res.json();
            if (data.components) {
                for (const key of REGION_KEYS) {
                    const comp = data.components[key];
                    if (comp) updateGauge(key, comp.score || 0);
                }
                if (data.decision_score !== undefined) {
                    document.getElementById('decision-score').textContent = data.decision_score.toFixed(3);
                    const badge = document.getElementById('decision-badge');
                    if (data.should_remember) {
                        badge.textContent = '已记忆';
                        badge.className = 'text-xs px-2 py-0.5 rounded-full badge-remember';
                    } else {
                        badge.textContent = '已拒绝';
                        badge.className = 'text-xs px-2 py-0.5 rounded-full badge-reject';
                    }
                }
            }
        } catch (e) { console.error('fetchBrainStatus error:', e); }
    }

    function updateGauge(key, score) {
        scores[key] = score;
        const pct = Math.min(score * 100, 100);
        document.getElementById('gauge-' + key).style.width = pct + '%';
        document.getElementById('score-' + key).textContent = score.toFixed(2);
    }

    async function doGate(execute) {
        const input = document.getElementById('gate-input');
        const message = input.value.trim();
        if (!message) return;

        const endpoint = execute ? '/api/gate/execute' : '/api/gate';
        try {
            const res = await fetch(endpoint, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ message }),
            });
            const data = await res.json();

            if (data.components) {
                for (const key of REGION_KEYS) {
                    const comp = data.components[key];
                    if (comp) updateGauge(key, comp.score || 0);
                }
            }

            const ds = data.decision_score || 0;
            document.getElementById('decision-score').textContent = ds.toFixed(3);
            const badge = document.getElementById('decision-badge');
            if (data.should_remember) {
                badge.textContent = '已记忆';
                badge.className = 'text-xs px-2 py-0.5 rounded-full badge-remember';
            } else {
                badge.textContent = '已拒绝';
                badge.className = 'text-xs px-2 py-0.5 rounded-full badge-reject';
            }

            const components = {};
            for (const key of REGION_KEYS) {
                components[key] = data.components?.[key]?.score || 0;
            }
            triggerGateAnimation(components, data.should_remember);
            showGateResult(data);

            if (execute && data.should_remember) {
                setTimeout(() => { fetchStats(); loadEngrams('L1'); }, 500);
            }
        } catch (e) { console.error('gate error:', e); }
    }

    function showGateResult(data) {
        const overlay = document.getElementById('gate-overlay');
        const title = document.getElementById('gate-result-title');
        const body = document.getElementById('gate-result-body');
        const card = document.getElementById('gate-result-card');

        title.textContent = data.should_remember ? '已记忆' : '已拒绝';
        title.style.color = data.should_remember ? '#4ade80' : '#f87171';
        card.style.borderColor = data.should_remember ? 'rgba(74,222,128,0.3)' : 'rgba(248,113,113,0.3)';

        let html = `<p>评分: <strong>${(data.decision_score || 0).toFixed(3)}</strong></p>`;
        html += `<p>重要度: ${data.importance || 0} | 情感: ${data.emotion || 'neutral'}</p>`;
        html += `<p class="text-gray-400">${data.reason || ''}</p>`;
        body.innerHTML = html;

        overlay.classList.remove('hiding');
        overlay.classList.add('visible');

        setTimeout(() => {
            overlay.classList.add('hiding');
            setTimeout(() => {
                overlay.classList.remove('visible');
                overlay.classList.remove('hiding');
            }, 500);
        }, 3000);
    }

    async function doSearch() {
        const input = document.getElementById('search-input');
        const query = input.value.trim();
        if (!query) return;

        try {
            const res = await fetch('/api/recall', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ query, top_k: 8 }),
            });
            const data = await res.json();
            const results = data.results || [];
            const container = document.getElementById('search-results');
            container.innerHTML = '';

            for (const r of results.slice(0, 8)) {
                const div = document.createElement('div');
                div.className = 'search-result';
                const content = r.content || '';
                const preview = content.length > 120 ? content.substring(0, 120) + '...' : content;
                div.innerHTML = `<div class="flex justify-between items-center">
                    <span class="text-cyan-400">[${r.layer || '?'}] ${(r.score || 0).toFixed(3)}</span>
                    <span class="text-gray-600 text-xs">imp:${r.importance || 0}</span>
                </div>
                <div class="text-gray-300 mt-1">${escapeHtml(preview)}</div>`;
                container.appendChild(div);
            }

            if (results.length === 0) {
                container.innerHTML = '<div class="text-gray-500">未找到相关记忆</div>';
            }
        } catch (e) { console.error('search error:', e); }
    }

    async function loadEngrams(layer) {
        try {
            const res = await fetch(`/api/engrams?layer=${layer}&limit=20`);
            const data = await res.json();
            const engrams = data.engrams || [];
            const container = document.getElementById('engram-feed');
            container.innerHTML = '';

            for (const e of engrams) {
                const div = document.createElement('div');
                div.className = 'memory-card';
                const content = e.content || '';
                const preview = content.length > 150 ? content.substring(0, 150) + '...' : content;
                const tags = (e.tags || []).map(t => `<span class="px-1 py-0.5 bg-gray-700 rounded text-xs">${t}</span>`).join(' ');
                div.innerHTML = `<div class="flex justify-between items-center">
                    <span class="text-xs text-gray-500">${e.created_at || ''}</span>
                    <span class="text-xs text-gray-500">imp:${e.importance || 0}</span>
                </div>
                <div class="text-sm text-gray-300 mt-1">${escapeHtml(preview)}</div>
                ${tags ? '<div class="flex gap-1 mt-1 flex-wrap">' + tags + '</div>' : ''}`;
                container.appendChild(div);
            }

            if (engrams.length === 0) {
                container.innerHTML = '<div class="text-gray-600 text-sm">' + layer + ' 层暂无记忆</div>';
            }
        } catch (e) { console.error('loadEngrams error:', e); }
    }

    // --- WebSocket ---
    let ws, wsReconnectTimer;

    function connectWS() {
        const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
        ws = new WebSocket(`${protocol}//${location.host}/api/events`);

        ws.onopen = () => updateWsStatus(true);
        ws.onmessage = (event) => {
            try { handleWsMessage(JSON.parse(event.data)); }
            catch (e) { console.error('ws parse error:', e); }
        };
        ws.onclose = () => {
            updateWsStatus(false);
            wsReconnectTimer = setTimeout(connectWS, 3000);
        };
        ws.onerror = () => ws.close();
    }

    function updateWsStatus(connected) {
        const el = document.getElementById('ws-status');
        if (connected) {
            el.innerHTML = '<span class="w-2 h-2 rounded-full bg-green-500 inline-block"></span> Connected';
        } else {
            el.innerHTML = '<span class="w-2 h-2 rounded-full bg-gray-600 inline-block"></span> Disconnected';
        }
    }

    function handleWsMessage(data) {
        switch (data.type) {
            case 'init':
                if (data.by_layer) {
                    document.getElementById('stat-l1').textContent = data.by_layer.L1 || 0;
                    document.getElementById('stat-l2').textContent = data.by_layer.L2 || 0;
                    document.getElementById('stat-l3').textContent = data.by_layer.L3 || 0;
                    document.getElementById('stat-total').textContent = data.total || 0;
                }
                break;
            case 'gate':
            case 'gate_execute':
                if (data.components) {
                    for (const key of REGION_KEYS) {
                        updateGauge(key, data.components[key] || 0);
                    }
                    triggerGateAnimation(data.components, data.should_remember);
                    document.getElementById('decision-score').textContent = (data.decision_score || 0).toFixed(3);
                    const badge = document.getElementById('decision-badge');
                    if (data.should_remember) {
                        badge.textContent = '已记忆';
                        badge.className = 'text-xs px-2 py-0.5 rounded-full badge-remember';
                    } else {
                        badge.textContent = '已拒绝';
                        badge.className = 'text-xs px-2 py-0.5 rounded-full badge-reject';
                    }
                }
                if (data.type === 'gate_execute') {
                    setTimeout(() => fetchStats(), 500);
                }
                break;
            case 'lagged':
                fetchStats();
                fetchBrainStatus();
                break;
        }
    }

    function escapeHtml(str) {
        const div = document.createElement('div');
        div.textContent = str;
        return div.innerHTML;
    }

    window.doGate = doGate;
    window.doSearch = doSearch;
    window.loadEngrams = loadEngrams;

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
