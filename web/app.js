// Hippocampus Gateway — Three.js 3D Brain Visualization
(function () {
    'use strict';

    // --- Brain Region Config ---
    const REGIONS = {
        amygdala: {
            color: 0xff4444, emissive: 0x661111,
            position: [0, -0.6, 0.6], radius: 0.45,
            weight: 0.35, label: 'Amygdala',
        },
        hippocampus: {
            color: 0xffd700, emissive: 0x665500,
            position: [0, 0.25, 0], radius: 0.55,
            weight: 0.30, label: 'Hippocampus',
        },
        prefrontal: {
            color: 0x4488ff, emissive: 0x112266,
            position: [0, 1.3, 1.1], radius: 0.50,
            weight: 0.20, label: 'Prefrontal',
        },
        temporal: {
            color: 0x44ff88, emissive: 0x116633,
            position: [-1.3, 0.2, -0.2], radius: 0.42,
            weight: 0.15, label: 'Temporal',
        },
    };

    const REGION_KEYS = ['amygdala', 'hippocampus', 'prefrontal', 'temporal'];

    // --- Three.js Setup ---
    let scene, camera, renderer, clock;
    let brainMeshes = {};
    let brainGlows = {};
    let connectionLines = [];
    let particles;
    let particlePositions, particleVelocities;
    const PARTICLE_COUNT = 300;

    // State
    let animationState = 'idle'; // idle | activating | high_score | writing | rejected
    let activationQueue = [];
    let activationTimer = 0;
    let breathPhase = 0;
    let scores = { amygdala: 0, hippocampus: 0, prefrontal: 0, temporal: 0 };

    function init() {
        scene = new THREE.Scene();
        scene.background = new THREE.Color(0x050510);
        scene.fog = new THREE.FogExp2(0x050510, 0.15);

        camera = new THREE.PerspectiveCamera(55, 1, 0.1, 100);
        camera.position.set(0, 0.5, 4.5);
        camera.lookAt(0, 0.3, 0.2);

        renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
        renderer.setPixelRatio(window.devicePixelRatio);
        const container = document.getElementById('canvas-container');
        renderer.setSize(container.clientWidth, container.clientHeight);
        container.appendChild(renderer.domElement);

        clock = new THREE.Clock();

        setupLighting();
        createBrainRegions();
        createConnections();
        createParticles();

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
        const w = container.clientWidth;
        const h = container.clientHeight;
        camera.aspect = w / h;
        camera.updateProjectionMatrix();
        renderer.setSize(w, h);
    }

    // --- Lighting ---
    function setupLighting() {
        const ambient = new THREE.AmbientLight(0x222244, 0.8);
        scene.add(ambient);

        const dir = new THREE.DirectionalLight(0xffffff, 0.6);
        dir.position.set(3, 5, 4);
        scene.add(dir);

        const back = new THREE.PointLight(0x4444ff, 0.4, 10);
        back.position.set(-3, -2, -3);
        scene.add(back);

        // Region point lights
        for (const key of REGION_KEYS) {
            const r = REGIONS[key];
            const light = new THREE.PointLight(r.color, 0.3, 3);
            light.position.set(r.position[0], r.position[1], r.position[2]);
            scene.add(light);
        }
    }

    // --- Brain Regions ---
    function createBrainRegions() {
        for (const key of REGION_KEYS) {
            const r = REGIONS[key];

            // Main sphere
            const geo = new THREE.SphereGeometry(r.radius, 32, 32);
            const mat = new THREE.MeshPhongMaterial({
                color: r.color,
                emissive: r.emissive,
                transparent: true,
                opacity: 0.65,
                shininess: 80,
            });
            const mesh = new THREE.Mesh(geo, mat);
            mesh.position.set(r.position[0], r.position[1], r.position[2]);
            mesh.userData = { key, baseScale: 1 };
            scene.add(mesh);
            brainMeshes[key] = mesh;

            // Glow sphere (larger, more transparent)
            const glowGeo = new THREE.SphereGeometry(r.radius * 1.6, 24, 24);
            const glowMat = new THREE.MeshBasicMaterial({
                color: r.color,
                transparent: true,
                opacity: 0.08,
            });
            const glowMesh = new THREE.Mesh(glowGeo, glowMat);
            glowMesh.position.copy(mesh.position);
            scene.add(glowMesh);
            brainGlows[key] = glowMesh;
        }
    }

    // --- Connections ---
    function createConnections() {
        const pairs = [
            ['amygdala', 'hippocampus'],
            ['hippocampus', 'prefrontal'],
            ['prefrontal', 'temporal'],
            ['amygdala', 'prefrontal'],
            ['hippocampus', 'temporal'],
            ['amygdala', 'temporal'],
        ];

        for (const [a, b] of pairs) {
            const ra = REGIONS[a];
            const rb = REGIONS[b];
            const thickness = (ra.weight + rb.weight) / 2;

            const points = [
                new THREE.Vector3(...ra.position),
                new THREE.Vector3(...rb.position),
            ];
            const geo = new THREE.BufferGeometry().setFromPoints(points);
            const mat = new THREE.LineBasicMaterial({
                color: 0x4466aa,
                transparent: true,
                opacity: 0.15 + thickness * 0.5,
            });
            const line = new THREE.Line(geo, mat);
            line.userData = { thickness };
            scene.add(line);
            connectionLines.push(line);
        }
    }

    // --- Particles ---
    function createParticles() {
        const positions = new Float32Array(PARTICLE_COUNT * 3);
        const velocities = [];

        for (let i = 0; i < PARTICLE_COUNT; i++) {
            // Random position around brain center
            const angle = Math.random() * Math.PI * 2;
            const radius = 0.8 + Math.random() * 2.0;
            const y = (Math.random() - 0.5) * 3;

            positions[i * 3] = Math.cos(angle) * radius;
            positions[i * 3 + 1] = y;
            positions[i * 3 + 2] = Math.sin(angle) * radius;

            velocities.push({
                x: (Math.random() - 0.5) * 0.002,
                y: (Math.random() - 0.5) * 0.002,
                z: (Math.random() - 0.5) * 0.002,
            });
        }

        const geo = new THREE.BufferGeometry();
        geo.setAttribute('position', new THREE.BufferAttribute(positions, 3));

        const mat = new THREE.PointsMaterial({
            color: 0x6688cc,
            size: 0.03,
            transparent: true,
            opacity: 0.5,
            blending: THREE.AdditiveBlending,
            depthWrite: false,
        });

        particles = new THREE.Points(geo, mat);
        scene.add(particles);
        particlePositions = positions;
        particleVelocities = velocities;
    }

    // --- Animation Loop ---
    function animate() {
        requestAnimationFrame(animate);
        const dt = clock.getDelta();
        breathPhase += dt;

        // Breathing pulse
        for (const key of REGION_KEYS) {
            const mesh = brainMeshes[key];
            const glow = brainGlows[key];
            const baseScale = mesh.userData.baseScale;
            const breath = 1 + Math.sin(breathPhase * 0.8 + REGIONS[key].position[0]) * 0.03;
            const targetScale = baseScale * breath;

            mesh.scale.lerp(new THREE.Vector3(targetScale, targetScale, targetScale), 0.1);
            glow.scale.lerp(new THREE.Vector3(targetScale * 1.6, targetScale * 1.6, targetScale * 1.6), 0.1);

            // Emissive intensity based on score
            const score = scores[key] || 0;
            const targetEmissive = new THREE.Color(REGIONS[key].emissive).multiplyScalar(1 + score * 2);
            mesh.material.emissive.lerp(targetEmissive, 0.05);
            glow.material.opacity = 0.05 + score * 0.15;
        }

        // Connection pulse
        for (const line of connectionLines) {
            const pulse = 0.7 + Math.sin(breathPhase * 1.2 + line.userData.thickness * 5) * 0.3;
            line.material.opacity = (0.1 + line.userData.thickness * 0.4) * pulse;
        }

        // Particle drift
        const pos = particlePositions;
        for (let i = 0; i < PARTICLE_COUNT; i++) {
            pos[i * 3] += particleVelocities[i].x;
            pos[i * 3 + 1] += particleVelocities[i].y;
            pos[i * 3 + 2] += particleVelocities[i].z;

            // Bound
            const dist = Math.sqrt(pos[i * 3] ** 2 + pos[i * 3 + 1] ** 2 + pos[i * 3 + 2] ** 2);
            if (dist > 3.5) {
                particleVelocities[i].x *= -1;
                particleVelocities[i].y *= -1;
                particleVelocities[i].z *= -1;
            }
        }
        particles.geometry.attributes.position.needsUpdate = true;

        // Activation queue processing
        if (activationQueue.length > 0) {
            activationTimer -= dt;
            if (activationTimer <= 0) {
                const next = activationQueue.shift();
                activateRegion(next.key, next.score);
                activationTimer = 0.3; // 300ms between activations
            }
        }

        // Slow camera orbit
        const time = clock.elapsedTime;
        camera.position.x = Math.sin(time * 0.15) * 0.3;
        camera.position.z = 4.5 + Math.cos(time * 0.1) * 0.2;
        camera.lookAt(0, 0.3, 0.2);

        renderer.render(scene, camera);
    }

    // --- Region Activation Effects ---
    function activateRegion(key, score) {
        const mesh = brainMeshes[key];
        const glow = brainGlows[key];
        const targetScale = 1 + score * 0.8;
        mesh.userData.baseScale = targetScale;

        // Flash glow
        glow.material.opacity = 0.4;

        // High score: bigger + particle burst
        if (score > 0.5) {
            mesh.userData.baseScale = 1 + score * 1.2;
            particleBurst(REGIONS[key].position, REGIONS[key].color);
        }

        // Gradually return to normal
        setTimeout(() => {
            mesh.userData.baseScale = 1;
        }, 2000);
    }

    function particleBurst(position, color) {
        // Temporarily change some particles to burst from position
        const pos = particlePositions;
        const count = 20;
        for (let i = 0; i < count; i++) {
            const idx = Math.floor(Math.random() * PARTICLE_COUNT);
            pos[idx * 3] = position[0] + (Math.random() - 0.5) * 0.2;
            pos[idx * 3 + 1] = position[1] + (Math.random() - 0.5) * 0.2;
            pos[idx * 3 + 2] = position[2] + (Math.random() - 0.5) * 0.2;
            const speed = 0.01 + Math.random() * 0.02;
            particleVelocities[idx].x = (Math.random() - 0.5) * speed * 3;
            particleVelocities[idx].y = (Math.random() - 0.5) * speed * 3;
            particleVelocities[idx].z = (Math.random() - 0.5) * speed * 3;
        }
        particles.geometry.attributes.position.needsUpdate = true;
    }

    function showRejected() {
        // Flash all regions red
        for (const key of REGION_KEYS) {
            const mesh = brainMeshes[key];
            mesh.userData.baseScale = 0.8;
            mesh.material.emissive.set(0x440000);
            setTimeout(() => {
                mesh.userData.baseScale = 1;
                mesh.material.emissive.set(REGIONS[key].emissive);
            }, 500);
        }
    }

    function showWriting() {
        // Particles converge to center
        const pos = particlePositions;
        for (let i = 0; i < PARTICLE_COUNT; i++) {
            pos[i * 3] *= 0.95;
            pos[i * 3 + 1] *= 0.95;
            pos[i * 3 + 2] *= 0.95;
        }
        particles.geometry.attributes.position.needsUpdate = true;
    }

    // --- Gate Evaluation Sequence ---
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

    // --- API Calls ---
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
        } catch (e) {
            console.error('fetchStats error:', e);
        }
    }

    async function fetchBrainStatus() {
        try {
            const res = await fetch('/api/brain/status');
            const data = await res.json();
            if (data.components) {
                for (const key of REGION_KEYS) {
                    const comp = data.components[key];
                    if (comp) {
                        const score = comp.score || 0;
                        updateGauge(key, score);
                    }
                }
                if (data.decision_score !== undefined) {
                    document.getElementById('decision-score').textContent = data.decision_score.toFixed(3);
                    const badge = document.getElementById('decision-badge');
                    if (data.should_remember) {
                        badge.textContent = 'REMEMBER';
                        badge.className = 'text-xs px-2 py-0.5 rounded-full badge-remember';
                    } else {
                        badge.textContent = 'REJECTED';
                        badge.className = 'text-xs px-2 py-0.5 rounded-full badge-reject';
                    }
                }
            }
        } catch (e) {
            console.error('fetchBrainStatus error:', e);
        }
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

            // Update decision display
            const ds = data.decision_score || 0;
            document.getElementById('decision-score').textContent = ds.toFixed(3);
            const badge = document.getElementById('decision-badge');
            if (data.should_remember) {
                badge.textContent = 'REMEMBER';
                badge.className = 'text-xs px-2 py-0.5 rounded-full badge-remember';
            } else {
                badge.textContent = 'REJECTED';
                badge.className = 'text-xs px-2 py-0.5 rounded-full badge-reject';
            }

            // Trigger 3D animation
            const components = {};
            for (const key of REGION_KEYS) {
                components[key] = data.components?.[key]?.score || 0;
            }
            triggerGateAnimation(components, data.should_remember);

            // Show overlay
            showGateResult(data);

            // Refresh stats after write
            if (execute && data.should_remember) {
                setTimeout(() => { fetchStats(); loadEngrams('L1'); }, 500);
            }
        } catch (e) {
            console.error('gate error:', e);
        }
    }

    function showGateResult(data) {
        const overlay = document.getElementById('gate-overlay');
        const title = document.getElementById('gate-result-title');
        const body = document.getElementById('gate-result-body');
        const card = document.getElementById('gate-result-card');

        title.textContent = data.should_remember ? 'Remembered' : 'Rejected';
        title.style.color = data.should_remember ? '#4ade80' : '#f87171';
        card.style.borderColor = data.should_remember ? 'rgba(74,222,128,0.3)' : 'rgba(248,113,113,0.3)';

        let html = `<p>Score: <strong>${(data.decision_score || 0).toFixed(3)}</strong></p>`;
        html += `<p>Importance: ${data.importance || 0} | Emotion: ${data.emotion || 'neutral'}</p>`;
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
                container.innerHTML = '<div class="text-gray-500">No results found</div>';
            }
        } catch (e) {
            console.error('search error:', e);
        }
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
                container.innerHTML = '<div class="text-gray-600 text-sm">No memories in ' + layer + '</div>';
            }
        } catch (e) {
            console.error('loadEngrams error:', e);
        }
    }

    // --- WebSocket ---
    let ws;
    let wsReconnectTimer;

    function connectWS() {
        const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
        ws = new WebSocket(`${protocol}//${location.host}/api/events`);

        ws.onopen = () => {
            updateWsStatus(true);
        };

        ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                handleWsMessage(data);
            } catch (e) {
                console.error('ws parse error:', e);
            }
        };

        ws.onclose = () => {
            updateWsStatus(false);
            wsReconnectTimer = setTimeout(connectWS, 3000);
        };

        ws.onerror = () => {
            ws.close();
        };
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
                        badge.textContent = 'REMEMBER';
                        badge.className = 'text-xs px-2 py-0.5 rounded-full badge-remember';
                    } else {
                        badge.textContent = 'REJECTED';
                        badge.className = 'text-xs px-2 py-0.5 rounded-full badge-reject';
                    }
                }
                if (data.type === 'gate_execute') {
                    setTimeout(() => { fetchStats(); }, 500);
                }
                break;

            case 'lagged':
                console.warn('WS lagged, missed', data.missed, 'events');
                fetchStats();
                fetchBrainStatus();
                break;
        }
    }

    // --- Utility ---
    function escapeHtml(str) {
        const div = document.createElement('div');
        div.textContent = str;
        return div.innerHTML;
    }

    // --- Expose to window ---
    window.doGate = doGate;
    window.doSearch = doSearch;
    window.loadEngrams = loadEngrams;

    // --- Init on load ---
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
