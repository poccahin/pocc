import React, { Suspense, useEffect, useMemo, useRef, useState } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { OrbitControls, Stars } from '@react-three/drei';
import * as THREE from 'three';

const particleVertexShader = `
  uniform float uTime;
  attribute float aScale;
  attribute vec3 aVelocity;

  uniform vec4 uAttractors[5];
  uniform int uAttractorCount;

  varying vec3 vColor;

  void main() {
    vec3 pos = position;
    pos += aVelocity * sin(uTime * 0.1 + pos.x * 0.5) * 0.02;

    vec3 idleColor = vec3(0.1, 0.3, 0.6);
    float collapseInfluence = 0.0;

    for (int i = 0; i < 5; i++) {
      if (i >= uAttractorCount) break;

      vec3 attractorPos = uAttractors[i].xyz;
      float frictionInverse = uAttractors[i].w;
      float dist = distance(pos, attractorPos);
      float radius = 2.5 * frictionInverse;

      if (dist < radius) {
        vec3 direction = normalize(attractorPos - pos);
        float pullStrength = pow(1.0 - dist / radius, 2.0) * frictionInverse * 0.8;
        pos += direction * pullStrength;
        collapseInfluence += pullStrength;
      }
    }

    vColor = mix(idleColor, vec3(1.0, 0.8, 0.2), min(collapseInfluence * 2.0, 1.0));

    vec4 mvPosition = modelViewMatrix * vec4(pos, 1.0);
    gl_PointSize = aScale * (300.0 / -mvPosition.z);
    gl_Position = projectionMatrix * mvPosition;
  }
`;

const particleFragmentShader = `
  varying vec3 vColor;

  void main() {
    float dist = length(gl_PointCoord - vec2(0.5));
    if (dist > 0.5) discard;

    float alpha = 1.0 - smoothstep(0.3, 0.5, dist);
    gl_FragColor = vec4(vColor, alpha * 0.8);
  }
`;

function StardustSystem({ count = 25000 }: { count?: number }) {
  const meshRef = useRef<THREE.Points>(null);
  const shaderRef = useRef<THREE.ShaderMaterial>(null);
  const [attractors, setAttractors] = useState<THREE.Vector4[]>([]);

  const particles = useMemo(() => {
    const positions = new Float32Array(count * 3);
    const velocities = new Float32Array(count * 3);
    const scales = new Float32Array(count);

    for (let i = 0; i < count; i += 1) {
      const r = 10 * Math.cbrt(Math.random());
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);

      positions[i * 3] = r * Math.sin(phi) * Math.cos(theta);
      positions[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta);
      positions[i * 3 + 2] = r * Math.cos(phi);

      velocities[i * 3] = (Math.random() - 0.5) * 0.2;
      velocities[i * 3 + 1] = (Math.random() - 0.5) * 0.2;
      velocities[i * 3 + 2] = (Math.random() - 0.5) * 0.2;

      scales[i] = Math.random() * 0.5 + 0.1;
    }

    return { positions, velocities, scales };
  }, [count]);

  useEffect(() => {
    const interval = setInterval(() => {
      const newAttractors = Array.from({ length: Math.floor(Math.random() * 3) + 1 }, () => {
        return new THREE.Vector4(
          (Math.random() - 0.5) * 15,
          (Math.random() - 0.5) * 15,
          (Math.random() - 0.5) * 15,
          Math.random() * 2 + 1.0,
        );
      });

      setAttractors(newAttractors);

      const clearTimer = setTimeout(() => {
        setAttractors([]);
      }, 1500);

      return () => clearTimeout(clearTimer);
    }, 3000);

    return () => clearInterval(interval);
  }, []);

  const uniforms = useMemo(
    () => ({
      uTime: { value: 0 },
      uAttractorCount: { value: 0 },
      uAttractors: {
        value: Array.from({ length: 5 }, () => new THREE.Vector4(0, 0, 0, 0)),
      },
    }),
    [],
  );

  useFrame((state) => {
    if (!shaderRef.current || !meshRef.current) {
      return;
    }

    shaderRef.current.uniforms.uTime.value = state.clock.getElapsedTime();
    shaderRef.current.uniforms.uAttractorCount.value = attractors.length;

    const attractorBuffer = shaderRef.current.uniforms.uAttractors.value as THREE.Vector4[];
    for (let i = 0; i < 5; i += 1) {
      attractorBuffer[i].copy(attractors[i] ?? new THREE.Vector4(0, 0, 0, 0));
    }

    meshRef.current.rotation.y = state.clock.getElapsedTime() * 0.02;
  });

  return (
    <points ref={meshRef}>
      <bufferGeometry>
        <bufferAttribute attach="attributes-position" args={[particles.positions, 3]} />
        <bufferAttribute attach="attributes-aVelocity" args={[particles.velocities, 3]} />
        <bufferAttribute attach="attributes-aScale" args={[particles.scales, 1]} />
      </bufferGeometry>
      <shaderMaterial
        ref={shaderRef}
        depthWrite={false}
        blending={THREE.AdditiveBlending}
        vertexShader={particleVertexShader}
        fragmentShader={particleFragmentShader}
        uniforms={uniforms}
        transparent
      />
    </points>
  );
}

function HUDOverlay() {
  return (
    <div className="absolute inset-0 z-10 flex select-none flex-col justify-between p-8 font-mono pointer-events-none">
      <header className="rounded-t-xl bg-gradient-to-b from-black/80 to-transparent px-4 pt-4 flex items-start justify-between">
        <div>
          <h1 className="text-3xl font-extrabold tracking-tighter text-transparent bg-clip-text bg-gradient-to-r from-emerald-400 to-blue-500">
            KOALA OS
          </h1>
          <p className="mt-1 text-xs uppercase tracking-[0.3em] text-blue-300/80">Planetary Situational Awareness</p>
        </div>

        <div className="text-right">
          <div className="mb-1 flex items-center justify-end gap-2">
            <div className="h-2 w-2 rounded-full bg-green-500 animate-pulse" />
            <span className="text-xs text-green-400">AHIN MAINNET: STABLE</span>
          </div>
          <p className="text-3xl font-bold tracking-tight text-white tabular-nums">
            142,857 <span className="text-sm font-normal text-slate-400">Active Nodes</span>
          </p>
        </div>
      </header>

      <footer className="grid grid-cols-3 gap-4 rounded-xl border-t border-blue-900/50 bg-black/60 p-4 backdrop-blur-sm">
        <div>
          <h3 className="mb-1 text-[10px] uppercase tracking-widest text-blue-400">Real-time Tensor Ops</h3>
          <p className="text-2xl font-bold text-white tabular-nums">
            8.2 <span className="text-lg text-blue-300">PetaFLOPS</span>
          </p>
          <div className="mt-2 h-1 w-full overflow-hidden rounded-full bg-blue-950">
            <div className="h-full w-[85%] bg-blue-500 animate-pulse" />
          </div>
        </div>
        <div className="pl-4 border-l border-blue-900/50">
          <h3 className="mb-1 text-[10px] uppercase tracking-widest text-amber-400">POCC Collapses / Sec</h3>
          <p className="text-2xl font-bold text-white tabular-nums">
            1,294 <span className="text-lg text-amber-300">CPS</span>
          </p>
          <div className="mt-2 h-1 w-full overflow-hidden rounded-full bg-amber-950">
            <div className="h-full w-[60%] bg-amber-500" />
          </div>
        </div>
        <div className="pl-4 border-l border-blue-900/50">
          <h3 className="mb-1 text-[10px] uppercase tracking-widest text-emerald-400">L3 Settlement Volume (24h)</h3>
          <p className="text-2xl font-bold text-white tabular-nums">
            $4.52 <span className="text-lg text-emerald-300">M</span>
          </p>
          <p className="mt-2 text-[10px] text-slate-400">ZK-Compressed Batches: #88921</p>
        </div>
      </footer>
    </div>
  );
}

export default function HolographicMap() {
  return (
    <div className="relative w-screen h-screen overflow-hidden bg-[#030712]">
      <HUDOverlay />

      <Canvas camera={{ position: [0, 0, 18], fov: 50 }} gl={{ antialias: true, alpha: false }}>
        <color attach="background" args={['#030712']} />
        <fog attach="fog" args={['#030712', 15, 35]} />

        <Suspense fallback={null}>
          <group rotation={[0, 0, Math.PI / 6]}>
            <StardustSystem count={25000} />
          </group>
          <Stars radius={50} depth={50} count={3000} factor={4} saturation={0} fade speed={1} />
          <OrbitControls
            autoRotate
            autoRotateSpeed={0.2}
            enableZoom={false}
            enablePan={false}
            maxPolarAngle={Math.PI / 1.5}
            minPolarAngle={Math.PI / 3}
          />
        </Suspense>
      </Canvas>

      <div className="absolute bottom-0 left-0 w-full h-32 pointer-events-none bg-gradient-to-t from-blue-900/20 to-transparent" />
    </div>
  );
}
