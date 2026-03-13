import React, { useEffect, useMemo, useRef } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import * as THREE from 'three';

const vertexShader = `
  uniform float uTime;
  attribute float size;
  attribute vec3 color;
  attribute float activityLevel;

  varying vec3 vColor;
  varying float vActivity;

  void main() {
    vColor = color;
    vActivity = activityLevel;

    vec3 pos = position;
    pos.y += sin(uTime * 2.0 + pos.x * 5.0) * 0.05 * (1.0 - activityLevel);

    if (activityLevel > 0.5) {
      pos.z += cos(uTime * 10.0) * 0.2;
    }

    vec4 mvPosition = modelViewMatrix * vec4(pos, 1.0);
    gl_PointSize = size * (300.0 / -mvPosition.z) * (1.0 + activityLevel * 3.0);
    gl_Position = projectionMatrix * mvPosition;
  }
`;

const fragmentShader = `
  varying vec3 vColor;
  varying float vActivity;

  void main() {
    vec2 xy = gl_PointCoord.xy - vec2(0.5);
    float ll = length(xy);
    if (ll > 0.5) discard;

    vec3 finalColor = mix(vColor, vec3(1.0, 0.9, 0.4), vActivity);
    float alpha = (1.0 - (ll * 2.0)) * (0.3 + vActivity * 0.7);

    gl_FragColor = vec4(finalColor, alpha);
  }
`;

const MAX_NODES = 500_000;

function StardustNetwork() {
  const pointsRef = useRef<THREE.Points>(null);
  const materialRef = useRef<THREE.ShaderMaterial>(null);

  const { positions, colors, sizes, activities } = useMemo(() => {
    const positionsArray = new Float32Array(MAX_NODES * 3);
    const colorsArray = new Float32Array(MAX_NODES * 3);
    const sizesArray = new Float32Array(MAX_NODES);
    const activitiesArray = new Float32Array(MAX_NODES);

    const baseColor = new THREE.Color('#0ea5e9');

    for (let i = 0; i < MAX_NODES; i += 1) {
      const r = 40 * Math.cbrt(Math.random());
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);

      positionsArray[i * 3] = r * Math.sin(phi) * Math.cos(theta);
      positionsArray[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta);
      positionsArray[i * 3 + 2] = r * Math.cos(phi);

      colorsArray[i * 3] = baseColor.r;
      colorsArray[i * 3 + 1] = baseColor.g;
      colorsArray[i * 3 + 2] = baseColor.b;

      sizesArray[i] = Math.random() * 2 + 0.5;
      activitiesArray[i] = 0;
    }

    return {
      positions: positionsArray,
      colors: colorsArray,
      sizes: sizesArray,
      activities: activitiesArray,
    };
  }, []);

  useEffect(() => {
    const worker = new Worker(new URL('../workers/telemetryWorker.ts', import.meta.url));

    worker.onmessage = (event: MessageEvent<{ index: number; friction: number }>) => {
      if (!pointsRef.current) {
        return;
      }

      const { index, friction } = event.data;
      if (index < 0 || index >= MAX_NODES) {
        return;
      }

      const activityLevel = 1 - friction;
      const attr = pointsRef.current.geometry.attributes.activityLevel as THREE.BufferAttribute;
      attr.setX(index, activityLevel);
      attr.needsUpdate = true;
    };

    return () => worker.terminate();
  }, []);

  useFrame((state) => {
    if (materialRef.current) {
      materialRef.current.uniforms.uTime.value = state.clock.getElapsedTime();
    }

    if (pointsRef.current) {
      pointsRef.current.rotation.y = state.clock.getElapsedTime() * 0.05;
    }
  });

  return (
    <points ref={pointsRef}>
      <bufferGeometry>
        <bufferAttribute attach="attributes-position" args={[positions, 3]} usage={THREE.StaticDrawUsage} />
        <bufferAttribute attach="attributes-color" args={[colors, 3]} usage={THREE.StaticDrawUsage} />
        <bufferAttribute attach="attributes-size" args={[sizes, 1]} usage={THREE.StaticDrawUsage} />
        <bufferAttribute attach="attributes-activityLevel" args={[activities, 1]} usage={THREE.DynamicDrawUsage} />
      </bufferGeometry>
      <shaderMaterial
        ref={materialRef}
        vertexShader={vertexShader}
        fragmentShader={fragmentShader}
        uniforms={{ uTime: { value: 0 } }}
        transparent
        depthWrite={false}
        blending={THREE.AdditiveBlending}
      />
    </points>
  );
}

export default function HolographicMap() {
  return (
    <div className="relative h-screen w-screen overflow-hidden bg-black font-mono text-cyan-500">
      <div className="absolute inset-0 z-0">
        <Canvas camera={{ position: [0, 15, 60], fov: 45 }}>
          <color attach="background" args={['#000000']} />
          <StardustNetwork />
        </Canvas>
      </div>

      <div className="pointer-events-none absolute inset-0 z-10 flex flex-col justify-between p-6">
        <header className="flex items-start justify-between">
          <div>
            <h1 className="drop-shadow-[0_0_10px_rgba(0,255,255,0.8)] text-4xl font-bold tracking-tighter text-white">
              KOALA OS <span className="text-sm font-normal text-cyan-600">v3.1.0</span>
            </h1>
            <p className="mt-1 text-xs tracking-widest opacity-70">PLANETARY ECOLOGY TELEMETRY</p>
          </div>
          <div className="text-right">
            <p className="text-2xl font-bold text-white">142,857,092</p>
            <p className="text-xs uppercase opacity-70">Active Silicon Entities</p>
          </div>
        </header>

        <footer className="pointer-events-auto grid grid-cols-3 gap-8 rounded-lg border border-cyan-900/50 bg-black/40 p-4 backdrop-blur-md">
          <div>
            <p className="text-xs opacity-70">GLOBAL SEMANTIC FRICTION (ℱ)</p>
            <p className="mt-1 text-xl font-bold text-emerald-400">
              0.0312 <span className="text-xs opacity-50">AVG</span>
            </p>
          </div>
          <div>
            <p className="text-xs opacity-70">x402 MICROPAYMENTS (TPS)</p>
            <p className="mt-1 text-xl font-bold text-amber-400">
              2.4M <span className="text-xs opacity-50">/ SEC</span>
            </p>
          </div>
          <div>
            <p className="text-xs opacity-70">ZK-ROLLUP BATCH COMPRESSION</p>
            <p className="mt-1 text-xl font-bold text-white">
              99.998% <span className="text-xs opacity-50">SAVED</span>
            </p>
          </div>
        </footer>
      </div>
    </div>
  );
}
