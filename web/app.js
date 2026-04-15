// Hippocampus Gateway — Neural Lightning Particle Engine (Final Visibility Fix)
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { EffectComposer } from 'three/addons/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/addons/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/addons/postprocessing/UnrealBloomPass.js';
import { OutputPass } from 'three/addons/postprocessing/OutputPass.js';

(function () {
    'use strict';

    const REGIONS = {
        amygdala: { color: new THREE.Color(0xff4444), pos: [0, -0.4, 0.2], label: '杏仁核' },
        hippocampus: { color: new THREE.Color(0xffd700), pos: [0, 0, 0.6], label: '海马体' },
        prefrontal: { color: new THREE.Color(0x4488ff), pos: [0, 0.6, 1.0], label: '前额叶' },
        temporal: { color: new THREE.Color(0x44ff88), pos: [-1.2, -0.1, 0.0], label: '颞叶' },
    };
    const REGION_KEYS = Object.keys(REGIONS);
    const PARTICLE_COUNT = 15000;

    let scene, camera, renderer, composer, controls, clock;
    let brainParticles, particleMaterial, brainGroup, sharedTexture;
    let regionNodes = {}; 
    let sparks = []; 
    let scores = { amygdala: 0, hippocampus: 0, prefrontal: 0, temporal: 0 };
    let currentDecisionScore = 0;
    let isFocusMode = false, isAudioOn = false;
    
    // --- Timer Management for Overlay ---
    let typewriterTimeout = null;
    let overlayCloseTimeout = null;
    let typeSoundTimer = null;

    // --- Shader Definitions ---
    const vertexShader = `
        attribute vec4 aWeights;
        attribute float aRandom;
        attribute vec3 color;
        
        uniform float uTime;
        uniform float uScores[4];
        
        varying vec3 vColor;
        varying float vAlpha;

        void main() {
            vec4 pos = vec4(position, 1.0);
            
            // Organic breathing
            float ripple = sin(uTime * 0.5 + pos.y * 1.5) * 0.04;
            pos.xyz += ripple;

            // Region activity
            float activeScore = 0.0;
            activeScore += aWeights.x * uScores[0];
            activeScore += aWeights.y * uScores[1];
            activeScore += aWeights.z * uScores[2];
            activeScore += aWeights.w * uScores[3];

            // Slower electrical flicker
            float flash = sin(uTime * (6.0 + aRandom * 8.0)) * 0.5 + 0.5;
            // EXTREME boost for active state (6.0 factor)
            float intensity = activeScore * flash * 6.0;
            
            // Dim background (color * 0.8), Pure white burst for active
            vColor = mix(color * 0.8, vec3(1.0, 1.0, 1.0), intensity * 0.95);
            // Lower base alpha (0.5), High active alpha
            vAlpha = 0.5 + intensity * 0.5;

            vec4 mvPosition = modelViewMatrix * pos;
            gl_PointSize = (36.0 * (1.0 + intensity * 0.5)) / -mvPosition.z;
            gl_Position = projectionMatrix * mvPosition;
        }
    `;

    const fragmentShader = `
        varying vec3 vColor;
        varying float vAlpha;
        uniform sampler2D uMap;
        void main() {
            vec4 texColor = texture2D(uMap, gl_PointCoord);
            gl_FragColor = vec4(vColor * texColor.rgb, vAlpha * texColor.a);
        }
    `;

    function createCircleTexture() {
        const canvas = document.createElement('canvas');
        canvas.width = 64; canvas.height = 64;
        const ctx = canvas.getContext('2d');
        const grad = ctx.createRadialGradient(32, 32, 0, 32, 32, 32);
        grad.addColorStop(0, 'rgba(255, 255, 255, 1)');
        grad.addColorStop(0.3, 'rgba(255, 255, 255, 0.8)');
        grad.addColorStop(0.6, 'rgba(255, 255, 255, 0.2)');
        grad.addColorStop(1, 'rgba(255, 255, 255, 0)');
        ctx.fillStyle = grad;
        ctx.fillRect(0, 0, 64, 64);
        return new THREE.CanvasTexture(canvas);
    }

    function init() {
        const canvasContainer = document.getElementById('canvas-container');
        
        // --- WebGL Support Check ---
        const testCanvas = document.createElement('canvas');
        const gl = testCanvas.getContext('webgl') || testCanvas.getContext('experimental-webgl');
        if (!gl) {
            console.error('WebGL not supported on this browser/hardware.');
            canvasContainer.innerHTML = '<div class="flex items-center justify-center h-full text-red-500 font-mono">ERROR: WEBGL_NOT_SUPPORTED</div>';
            return;
        }

        console.log('Hippocampus 3D Engine Initializing...');
        clock = new THREE.Clock();
        scene = new THREE.Scene();
        scene.background = new THREE.Color(0x020205);

        camera = new THREE.PerspectiveCamera(30, window.innerWidth / window.innerHeight, 0.1, 1000);
        const isMobile = window.innerWidth < 768;
        camera.position.set(0, isMobile ? 1.5 : 0.8, isMobile ? 10.0 : 7.0);

        renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: "high-performance" });
        renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
        renderer.setSize(window.innerWidth, window.innerHeight);
        document.getElementById('canvas-container').appendChild(renderer.domElement);

        sharedTexture = createCircleTexture();

        const renderScene = new RenderPass(scene, camera);
        const bloomPass = new UnrealBloomPass(new THREE.Vector2(window.innerWidth, window.innerHeight), 0.8, 0.4, 0.0); 
        
        composer = new EffectComposer(renderer);
        composer.addPass(renderScene);
        composer.addPass(bloomPass);
        composer.addPass(new OutputPass());

        controls = new OrbitControls(camera, renderer.domElement);
        controls.enableDamping = true;
        controls.autoRotate = true;
        controls.autoRotateSpeed = isMobile ? 0.1 : 0.2;
        controls.enablePan = false;

        createNeuralSystem();
        setupLights();

        window.addEventListener('resize', onWindowResize);

        // Mouse tracking for card glow effect
        document.addEventListener('mousemove', (e) => {
            document.querySelectorAll('.memory-card').forEach(card => {
                const rect = card.getBoundingClientRect();
                const x = ((e.clientX - rect.left) / rect.width) * 100;
                const y = ((e.clientY - rect.top) / rect.height) * 100;
                card.style.setProperty('--mouse-x', x + '%');
                card.style.setProperty('--mouse-y', y + '%');
            });
        });
        
        fetchStats(); fetchBrainStatus(); loadEngrams('L1'); connectWS();
        console.log('Hippocampus 3D Engine Ready.');
        animate();
    }

    function createNeuralSystem() {
        brainGroup = new THREE.Group();
        scene.add(brainGroup);

        const positions = [];
        const colors = [];
        const weights = [];
        const randoms = [];

        const regionCenters = REGION_KEYS.map(k => new THREE.Vector3(...REGIONS[k].pos));
        const regionColors = REGION_KEYS.map(k => REGIONS[k].color);

        for (let i = 0; i < PARTICLE_COUNT; i++) {
            const side = Math.random() > 0.5 ? 1 : -1;
            let x = (Math.random() - 0.5) * 2.8 * side;
            let y = (Math.random() - 0.5) * 2.4;
            let z = (Math.random() - 0.5) * 3.4;

            const dx = x / 1.35; const dy = y / 1.15; const dz = z / 1.55;
            if (dx * dx + dy * dy + dz * dz < 1.0) {
                positions.push(x, y, z);
                randoms.push(Math.random());

                const p = new THREE.Vector3(x, y, z);
                let w = [0, 0, 0, 0];
                let finalCol = new THREE.Color(0x0088ff); 

                regionCenters.forEach((center, idx) => {
                    const dist = p.distanceTo(center);
                    if (dist < 0.9) {
                        const influence = Math.pow(1.0 - dist / 0.9, 2.0);
                        w[idx] = influence;
                        finalCol.lerp(regionColors[idx], influence * 0.9);
                    }
                });

                colors.push(finalCol.r, finalCol.g, finalCol.b);
                weights.push(...w);
            }
        }

        const geo = new THREE.BufferGeometry();
        geo.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
        geo.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));
        geo.setAttribute('aWeights', new THREE.Float32BufferAttribute(weights, 4));
        geo.setAttribute('aRandom', new THREE.Float32BufferAttribute(randoms, 1));

        particleMaterial = new THREE.ShaderMaterial({
            uniforms: {
                uTime: { value: 0 },
                uScores: { value: [0, 0, 0, 0] },
                uMap: { value: sharedTexture }
            },
            vertexShader,
            fragmentShader,
            transparent: true,
            blending: THREE.AdditiveBlending,
            depthWrite: false
        });

        brainParticles = new THREE.Points(geo, particleMaterial);
        brainGroup.add(brainParticles);
    }

    function setupLights() {
        scene.add(new THREE.AmbientLight(0x444466, 1.0));
        const p1 = new THREE.PointLight(0x00f3ff, 2, 20); p1.position.set(5, 5, 5); scene.add(p1);
        const p2 = new THREE.PointLight(0xb534ff, 2, 20); p2.position.set(-5, -5, -5); scene.add(p2);
    }

    function animate() {
        requestAnimationFrame(animate);
        const delta = clock.getDelta();
        const elapsed = clock.getElapsedTime();
        controls.update();

        if (brainGroup) {
            brainGroup.rotation.y += 0.0008;
        }

        if (particleMaterial) {
            particleMaterial.uniforms.uTime.value = elapsed;
            particleMaterial.uniforms.uScores.value = [scores.amygdala, scores.hippocampus, scores.prefrontal, scores.temporal];
        }

        const decayRate = 0.12 * delta;
        REGION_KEYS.forEach((key) => {
            if (scores[key] > 0) {
                scores[key] = Math.max(0, scores[key] - decayRate);
                document.getElementById('gauge-' + key).style.width = Math.sqrt(scores[key]) * 100 + '%';
                document.getElementById('score-' + key).textContent = scores[key].toFixed(2);
            }
        });

        if (currentDecisionScore > 0) {
            currentDecisionScore = Math.max(0, currentDecisionScore - decayRate);
            const scoreEl = document.getElementById('decision-score');
            if (scoreEl) scoreEl.textContent = currentDecisionScore.toFixed(3);
            if (currentDecisionScore === 0) {
                const badge = document.getElementById('decision-badge');
                if (badge) {
                    badge.textContent = '待机';
                    badge.className = 'text-g5 px-2 py-0.5 rounded border border-gray-700 uppercase';
                }
            }
        }

        if (Math.random() < 0.08 && sparks.length < 15) spawnSpark();
        updateSparks(delta);

        composer.render();
    }

    function spawnSpark(startPos, endPos, color = 0x00f3ff) {
        const posAttr = brainParticles.geometry.attributes.position;
        if (!startPos) {
            const i1 = Math.floor(Math.random() * posAttr.count);
            const i2 = Math.floor(Math.random() * posAttr.count);
            startPos = new THREE.Vector3(posAttr.getX(i1), posAttr.getY(i1), posAttr.getZ(i1));
            endPos = new THREE.Vector3(posAttr.getX(i2), posAttr.getY(i2), posAttr.getZ(i2));
            if (startPos.distanceTo(endPos) > 1.0) return; 
        }

        const geo = new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(0,0,0)]);
        const mat = new THREE.PointsMaterial({
            color: color,
            size: 0.12,
            transparent: true,
            opacity: 0.5,
            map: sharedTexture,
            blending: THREE.AdditiveBlending,
            depthWrite: false
        });
        const point = new THREE.Points(geo, mat);
        brainGroup.add(point);
        
        sparks.push({ 
            mesh: point, start: startPos.clone(), end: endPos.clone(), progress: 0, 
            speed: 1.5 + Math.random() * 2.0 
        });
    }

    function updateSparks(dt) {
        for (let i = sparks.length - 1; i >= 0; i--) {
            const s = sparks[i];
            s.progress += dt * s.speed;
            if (s.progress >= 1.0) {
                brainGroup.remove(s.mesh);
                sparks.splice(i, 1);
            } else {
                s.mesh.position.lerpVectors(s.start, s.end, s.progress);
                s.mesh.material.opacity = Math.sin(Math.PI * s.progress) * 0.5;
                s.mesh.scale.setScalar(Math.sin(Math.PI * s.progress) * 1.5);
            }
        }
    }

    function triggerNeuralEvent(key, intensity) {
        const region = REGIONS[key];
        const center = new THREE.Vector3(...region.pos);
        const num = Math.floor(intensity * 25);
        for (let i = 0; i < num; i++) {
            const start = center.clone().add(new THREE.Vector3((Math.random()-0.5)*1.5, (Math.random()-0.5)*1.5, (Math.random()-0.5)*1.5));
            const end = center.clone().add(new THREE.Vector3((Math.random()-0.5)*0.4, (Math.random()-0.5)*0.4, (Math.random()-0.5)*0.4));
            spawnSpark(start, end, region.color);
        }
    }

    async function doGate(execute) {
        const input = document.getElementById('gate-input');
        const message = input.value.trim(); if (!message) return;
        const endpoint = execute ? '/api/gate/execute' : '/api/gate';
        try {
            const res = await fetch(endpoint, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ message }) });
            const data = await res.json();
            if (data.components) REGION_KEYS.forEach(k => updateGauge(k, data.components[k]?.score || 0));
            updateDecision(data.decision_score || 0, data.should_remember);
            showGateResult(data);
            if (execute && data.should_remember) { input.value = ''; setTimeout(() => { fetchStats(); loadEngrams('L1'); }, 1500); }
        } catch (e) {}
    }

    function updateGauge(key, score, suppress = false) {
        scores[key] = score;
        if (!suppress && score > 0.3) triggerNeuralEvent(key, score);
    }

    function updateDecision(score, remember) {
        currentDecisionScore = score;
        const scoreEl = document.getElementById('decision-score');
        if (scoreEl) scoreEl.textContent = score.toFixed(3);
        const badge = document.getElementById('decision-badge');
        if (badge) {
            badge.textContent = remember ? '已记忆' : '已拒绝';
            badge.className = 'text-g5 px-2 py-0.5 rounded border ' + (remember ? 'badge-remember' : 'badge-reject');
        }
    }

    function showGateResult(data) {
        const overlay = document.getElementById('gate-overlay');
        const body = document.getElementById('gate-result-body');
        const statusBadge = document.getElementById('gate-ready-text');
        
        // Clear all existing timers to prevent collision
        if (typewriterTimeout) clearTimeout(typewriterTimeout);
        if (overlayCloseTimeout) clearTimeout(overlayCloseTimeout);
        if (typeSoundTimer) {
            const typeSound = document.getElementById('sound-type');
            if (typeSound) { typeSound.loop = false; typeSound.pause(); }
        }

        overlay.classList.remove('hidden', 'hiding'); 
        overlay.classList.add('visible');
        
        if (statusBadge) {
            statusBadge.textContent = 'PROCESSING...';
            statusBadge.className = 'text-g5 uppercase font-mono bg-cyan-500/10 px-2 py-0.5 border border-cyan-500/30 text-cyan-400 animate-pulse';
        }

        // Reset content immediately
        if (body) body.innerHTML = '';

        const textToType = `>> 认知映射分析中...\n>> 置信分值: ${(data.decision_score||0).toFixed(4)}\n>> 重要程度: ${data.importance||0}\n>> 情感倾向: ${data.emotion || 'neutral'}\n\n[判断结果]: ${data.reason || '无特殊标记'}`;
        
        const charSpeed = 20; 
        const totalTypeTime = textToType.length * charSpeed;
        const readTime = 4000;

        setTimeout(() => {
            typeWriterEffect('gate-result-body', textToType, charSpeed);
        }, 100);

        // Track closure timer
        overlayCloseTimeout = setTimeout(() => {
            if (statusBadge) {
                statusBadge.textContent = 'ANALYSIS COMPLETE';
                statusBadge.classList.remove('animate-pulse');
                statusBadge.className = 'text-g5 uppercase font-mono bg-green-500/10 px-2 py-0.5 border border-green-500/30 text-green-400';
            }
            
            setTimeout(() => {
                overlay.classList.add('hiding');
                setTimeout(() => { 
                    overlay.classList.remove('visible', 'hiding'); 
                    overlay.classList.add('hidden'); 
                }, 600);
            }, readTime);
        }, totalTypeTime + 500);
    }

    async function fetchStats() {
        try {
            const res = await fetch('/api/stats');
            const data = await res.json();
            if (data.status === 'ok') {
                document.getElementById('stat-l1').textContent = data.by_layer?.L1 || 0;
                document.getElementById('stat-l2').textContent = data.by_layer?.L2 || 0;
                document.getElementById('stat-l3').textContent = data.by_layer?.L3 || 0;
                document.getElementById('stat-total').textContent = data.total_engrams || 0;
                const max = { L1: 500, L2: 5000, L3: 20000 };
                ['l1', 'l2', 'l3'].forEach(l => {
                    const el = document.getElementById(`spark-${l}`);
                    if (el) el.style.width = Math.min(((data.by_layer?.[l.toUpperCase()] || 0) / max[l.toUpperCase()]) * 100, 100) + '%';
                });
            }
        } catch (e) {}
    }

    async function fetchBrainStatus() {
        try {
            const res = await fetch('/api/brain/status');
            const data = await res.json();
            if (data.components) {
                REGION_KEYS.forEach(k => {
                    const el = document.getElementById('gauge-' + k); if (el) el.style.width = '0%';
                    const txt = document.getElementById('score-' + k); if (txt) txt.textContent = '0.00';
                    scores[k] = 0;
                });
                updateDecision(0, false);
            }
        } catch (e) {}
    }

    async function doSearch() {
        const input = document.getElementById('search-input');
        const query = input.value.trim(); if (!query) return;
        try {
            const res = await fetch('/api/recall', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ query, top_k: 5 }) });
            const data = await res.json();
            const container = document.getElementById('search-results'); container.innerHTML = '';
            (data.results || []).forEach((r, i) => {
                const div = document.createElement('div'); div.className = 'search-result'; div.style.animationDelay = `${i * 0.1}s`;
                div.innerHTML = `<div class="flex justify-between text-g5 font-mono mb-1.5"><span class="text-cyan-400/80">${r.layer} | ${(r.score || 0).toFixed(3)}</span><span>IMP:${r.importance}</span></div><div class="text-g2 text-gray-200 leading-normal">${highlightText(r.content, query)}</div>`;
                container.appendChild(div);
            });
        } catch (e) {}
    }

    async function loadEngrams(layer) {
        document.querySelectorAll('#layer-tabs button').forEach(b => { b.classList.remove('active'); if (b.textContent === layer) b.classList.add('active'); });
        try {
            const res = await fetch(`/api/engrams?layer=${layer}&limit=15`);
            const data = await res.json();
            const container = document.getElementById('engram-feed'); container.innerHTML = '';
            
            (data.engrams || []).forEach((e, i) => {
                const div = document.createElement('div');
                div.className = 'memory-card group';
                div.style.animationDelay = `${i * 0.05}s`;
                div.setAttribute('data-emotion', e.emotion || 'neutral');

                const dateStr = e.created_at.split('T')[0];
                const timeStr = e.created_at.split('T')[1]?.split('.')[0] || '';

                const tagsHtml = (e.tags || [])
                    .map(t => `<span class="engram-tag">${escapeHtml(t)}</span>`)
                    .join('');

                div.innerHTML = `
                    <div class="emotion-bar"></div>
                    <div class="engram-inner">
                        <div class="engram-header">
                            <div class="engram-meta">
                                <span class="engram-layer-badge">${e.layer || 'L1'}</span>
                                <span class="engram-time">${dateStr} ${timeStr}</span>
                            </div>
                            <div class="importance-ring" title="重要性: ${e.importance}" style="--importance: ${e.importance * 10}">
                                <svg viewBox="0 0 36 36">
                                    <circle class="importance-ring-bg" cx="18" cy="18" r="14"/>
                                    <circle class="importance-ring-fill" cx="18" cy="18" r="14"/>
                                </svg>
                                <span class="importance-ring-value">${Math.round(e.importance * 10)}</span>
                            </div>
                        </div>
                        <div class="engram-content">${escapeHtml(e.content)}</div>
                        <div class="engram-footer">
                            <div class="tag-list">${tagsHtml}</div>
                            <button onclick="openDeleteModal('${e.id}')" class="engram-delete-btn" title="永久删除此印迹">
                                <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path></svg>
                            </button>
                        </div>
                    </div>
                `;
                container.appendChild(div);
            });

        } catch (e) {
            console.error('Failed to load engrams:', e);
        }
    }

    let deleteTargetId = null;

    async function deleteEngram(id) {
        try {
            const res = await fetch(`/api/engrams/${id}`, { method: 'DELETE' });
            const data = await res.json();
            if (data.status === 'ok') {
                playSound('click');
                fetchStats();
                const activeTab = document.querySelector('#layer-tabs button.active');
                if (activeTab) loadEngrams(activeTab.textContent);
            }
        } catch (e) {
            console.error('Delete failed:', e);
        }
    }

    function openDeleteModal(id) {
        deleteTargetId = id;
        const container = document.getElementById('modal-container');
        const modal = document.getElementById('delete-modal');
        
        container.classList.remove('hidden');
        // Force reflow
        container.offsetHeight;
        
        container.classList.add('opacity-100');
        modal.classList.add('scale-100');
        modal.classList.remove('scale-95');
        
        const confirmBtn = document.getElementById('modal-confirm-btn');
        confirmBtn.onclick = async () => {
            if (deleteTargetId) {
                await deleteEngram(deleteTargetId);
                closeDeleteModal();
            }
        };
        playSound('hover');
    }

    function closeDeleteModal() {
        const container = document.getElementById('modal-container');
        const modal = document.getElementById('delete-modal');
        
        container.classList.remove('opacity-100');
        modal.classList.remove('scale-100');
        modal.classList.add('scale-95');
        
        setTimeout(() => {
            container.classList.add('hidden');
            deleteTargetId = null;
        }, 300);
    }

    function connectWS() {
        const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
        const ws = new WebSocket(`${protocol}//${location.host}/api/events`);
        const statusEl = document.getElementById('ws-status');
        ws.onopen = () => { if (statusEl) statusEl.innerHTML = '<span class="w-1.5 h-1.5 rounded-full bg-cyan-500 shadow-[0_0_5px_rgba(0,243,255,0.8)]"></span> 在线'; };
        ws.onclose = () => { if (statusEl) statusEl.innerHTML = '<span class="w-1.5 h-1.5 rounded-full bg-gray-600"></span> 离线'; setTimeout(connectWS, 5000); };
        ws.onmessage = (event) => {
            const data = JSON.parse(event.data);
            if (data.type === 'gate' || data.type === 'gate_execute') {
                if (data.components) REGION_KEYS.forEach(k => updateGauge(k, data.components[k] || 0));
                updateDecision(data.decision_score || 0, data.should_remember);
                
                if (data.type === 'gate_execute') {
                    // 延迟刷新数据，确保后端磁盘写入已完成
                    setTimeout(() => {
                        fetchStats();
                        const activeTab = document.querySelector('#layer-tabs button.active');
                        if (activeTab && activeTab.textContent === 'L1') loadEngrams('L1');
                    }, 800);
                }
            } else if (data.type === 'delete_engram') {
                console.log('Engram deleted neural event:', data.id);
                // 视觉反馈：红色闪烁
                triggerNeuralEvent('amygdala', 0.9);
                fetchStats();
                const activeTab = document.querySelector('#layer-tabs button.active');
                if (activeTab) loadEngrams(activeTab.textContent);
            } else if (data.type === 'hook_event') {
                // 兼容旧的 hook_event (如果有的话)
                console.log('Neural Hook received:', data.hook_type);
                const randomRegion = REGION_KEYS[Math.floor(Math.random() * REGION_KEYS.length)];
                triggerNeuralEvent(randomRegion, 0.8);
                setTimeout(() => {
                    fetchStats();
                    const activeTab = document.querySelector('#layer-tabs button.active');
                    if (activeTab && activeTab.textContent === 'L1') loadEngrams('L1');
                }, 1000);
            }
        };
    }

    function toggleFocusMode() {
        isFocusMode = !isFocusMode;
        document.getElementById('btn-focus').classList.toggle('active');
        document.getElementById('panel-left').classList.toggle('panel-hidden-left');
        document.getElementById('panel-right').classList.toggle('panel-hidden-right');
        document.getElementById('panel-bottom').classList.toggle('panel-hidden-bottom');
        
        // Immediate label hide
        REGION_KEYS.forEach(key => {
            if (regionNodes[key] && regionNodes[key].element) {
                regionNodes[key].element.style.opacity = isFocusMode ? 0 : 0.3;
            }
        });

        playSound('click');
    }

    function toggleAudio() {
        isAudioOn = !isAudioOn;
        const btn = document.getElementById('btn-audio');
        const ambient = document.getElementById('sound-ambient');
        btn.classList.toggle('active');
        if (isAudioOn) { btn.textContent = '环境音'; ambient.volume = 0.3; ambient.play().catch(()=>{}); }
        else { btn.textContent = '静音'; ambient.pause(); }
        playSound('click');
    }

    function typeWriterEffect(elementId, text, speed = 25) {
        const el = document.getElementById(elementId);
        if (!el) return;
        let i = 0;
        const typeSound = document.getElementById('sound-type');
        if (isAudioOn) { typeSound.volume = 0.2; typeSound.loop = true; typeSound.play().catch(()=>{}); }
        
        function type() {
            if (i < text.length) {
                el.innerHTML = text.substring(0, i + 1) + '<span class="typewriter-cursor"></span>';
                i++; 
                typewriterTimeout = setTimeout(type, speed + Math.random() * 15);
            } else {
                el.innerHTML = text;
                if (isAudioOn) { typeSound.loop = false; typeSound.pause(); }
                typewriterTimeout = null;
            }
        }
        type();
    }

    function playSound(id) {
        if (!isAudioOn && id !== 'hover') return;
        const sound = document.getElementById('sound-' + id);
        if (sound) { sound.currentTime = 0; sound.volume = id === 'hover' ? 0.05 : 0.25; sound.play().catch(() => {}); }
    }

    function highlightText(text, keyword) {
        if (!keyword) return escapeHtml(text);
        const escaped = escapeHtml(text);
        const regex = new RegExp(`(${escapeHtml(keyword)})`, 'gi');
        return escaped.replace(regex, '<span class="highlight-text">$1</span>');
    }

    function escapeHtml(str) { const div = document.createElement('div'); div.textContent = str; return div.innerHTML; }
    function onWindowResize() {
        camera.aspect = window.innerWidth / window.innerHeight; camera.updateProjectionMatrix();
        renderer.setSize(window.innerWidth, window.innerHeight); composer.setSize(window.innerWidth, window.innerHeight);
    }

    window.doGate = doGate; window.doSearch = doSearch; window.loadEngrams = loadEngrams;
    window.deleteEngram = deleteEngram;
    window.openDeleteModal = openDeleteModal;
    window.closeDeleteModal = closeDeleteModal;
    window.toggleFocusMode = toggleFocusMode; window.toggleAudio = toggleAudio;
    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init); else init();
})();
