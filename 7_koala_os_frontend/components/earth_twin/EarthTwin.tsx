import React, { useMemo, useRef } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { OrbitControls, Sphere } from '@react-three/drei';
import * as THREE from 'three';

import { PlanetaryPulse, useOmnisphere } from '../../hooks/useOmnisphere';

const latLonToVector3 = (lat: number, lon: number, radius: number): THREE.Vector3 => {
  const phi = (90 - lat) * (Math.PI / 180);
  const theta = (lon + 180) * (Math.PI / 180);

  return new THREE.Vector3(
    -(radius * Math.sin(phi) * Math.cos(theta)),
    radius * Math.cos(phi),
    radius * Math.sin(phi) * Math.sin(theta),
  );
};

const PulseNode: React.FC<{ pulse: PlanetaryPulse; radius: number }> = ({ pulse, radius }) => {
  const meshRef = useRef<THREE.Mesh>(null);
  const materialRef = useRef<THREE.MeshBasicMaterial>(null);

  const position = useMemo(
    () => latLonToVector3(pulse.gps[0], pulse.gps[1], radius),
    [pulse.gps, radius],
  );

  const isRed = pulse.type === 'TENSOR_INTERCEPTION' || pulse.type === 'LIFE_BURN';
  const color = isRed ? '#ff3344' : '#33ccff';
  const baseScale = pulse.type === 'LIFE_BURN' ? 1.6 : 0.85;

  useFrame(() => {
    if (!meshRef.current || !materialRef.current) {
      return;
    }

    const age = (Date.now() - pulse.receivedAt) / 2000;
    if (age > 1) {
      meshRef.current.visible = false;
      return;
    }

    meshRef.current.visible = true;
    meshRef.current.scale.setScalar(baseScale * (1 + age * 2.2));
    materialRef.current.opacity = 1 - age;
  });

  return (
    <mesh ref={meshRef} position={position}>
      <sphereGeometry args={[0.05 * baseScale, 24, 24]} />
      <meshBasicMaterial ref={materialRef} color={color} transparent opacity={1} depthWrite={false} />
    </mesh>
  );
};

export const EarthTwin: React.FC = () => {
  const { pulses, groupedCounts, isConnected } = useOmnisphere('ws://localhost:9000');
  const earthRadius = 2.5;

  return (
    <div className="w-full h-screen bg-black relative">
      <div className="absolute top-8 left-8 text-white z-10 font-mono pointer-events-none">
        <h1 className="text-3xl font-bold tracking-widest text-blue-400">KOALA OS :: OMNISPHERE</h1>
        <p className="text-sm opacity-70">
          Planetary Entropy Status: {isConnected ? 'ONLINE' : 'CONNECTING...'}
        </p>
        <p className="text-xs opacity-70 mt-2">
          Interceptions: {groupedCounts.TENSOR_INTERCEPTION} · LIFE Burn: {groupedCounts.LIFE_BURN} · Routed:{' '}
          {groupedCounts.ROUTING_SUCCESS}
        </p>
      </div>

      <Canvas camera={{ position: [0, 0, 6], fov: 50 }}>
        <ambientLight intensity={0.25} />
        <pointLight position={[8, 8, 8]} intensity={1.4} />

        <Sphere args={[earthRadius, 64, 64]}>
          <meshStandardMaterial color="#1a1a3a" roughness={0.8} metalness={0.2} />
        </Sphere>

        <Sphere args={[earthRadius + 0.01, 64, 64]}>
          <meshBasicMaterial color="#5566ff" wireframe transparent opacity={0.12} />
        </Sphere>

        {pulses.map((pulse) => (
          <PulseNode key={pulse.id} pulse={pulse} radius={earthRadius + 0.02} />
        ))}

        <OrbitControls enableZoom enablePan autoRotate autoRotateSpeed={0.45} />
      </Canvas>
    </div>
  );
};
