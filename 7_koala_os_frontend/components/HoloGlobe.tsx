import React, { useMemo, useRef } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { Sphere, Stars } from '@react-three/drei';
import * as THREE from 'three';

type ComputeNodeType = 'M4' | 'AMD';

type ComputeNodeProps = {
  position: [number, number, number];
  type: ComputeNodeType;
  intensity: number;
};

type FlowArc = {
  start: THREE.Vector3;
  end: THREE.Vector3;
  type: ComputeNodeType;
};

const arcVertexShader = `
  varying vec2 vUv;

  void main() {
    vUv = uv;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

const arcFragmentShader = `
  uniform float time;
  uniform vec3 color;
  varying vec2 vUv;

  void main() {
    float pulse = smoothstep(0.4, 0.5, sin(vUv.x * 10.0 - time * 5.0) * 0.5 + 0.5);
    gl_FragColor = vec4(color, pulse * 0.8);
  }
`;

const ComputeNode = ({ position, type, intensity }: ComputeNodeProps) => {
  const color = type === 'M4' ? '#00ff66' : '#0066ff';

  return (
    <mesh position={position}>
      <sphereGeometry args={[0.02 * intensity, 16, 16]} />
      <meshBasicMaterial color={color} />
      <pointLight distance={0.5} intensity={intensity} color={color} />
    </mesh>
  );
};

const EarthScene = () => {
  const globeRef = useRef<THREE.Group>(null);
  const arcMaterialsRef = useRef<THREE.ShaderMaterial[]>([]);

  const flowArcs = useMemo<FlowArc[]>(() => {
    return Array.from({ length: 40 }).map(() => ({
      start: new THREE.Vector3().setFromSphericalCoords(2, Math.random() * Math.PI, Math.random() * Math.PI * 2),
      end: new THREE.Vector3().setFromSphericalCoords(2, Math.random() * Math.PI, Math.random() * Math.PI * 2),
      type: Math.random() > 0.5 ? 'M4' : 'AMD',
    }));
  }, []);

  useFrame(({ clock }) => {
    if (globeRef.current) {
      globeRef.current.rotation.y += 0.002;
    }

    const elapsed = clock.getElapsedTime();
    for (const material of arcMaterialsRef.current) {
      material.uniforms.time.value = elapsed;
    }
  });

  return (
    <group ref={globeRef}>
      <Sphere args={[2, 64, 64]}>
        <meshPhongMaterial color="#050505" emissive="#001111" specular="#111111" shininess={10} wireframe={false} />
      </Sphere>

      <Sphere args={[2.01, 40, 40]}>
        <meshBasicMaterial color="#00ffcc" wireframe opacity={0.05} transparent />
      </Sphere>

      {flowArcs.map((arc, index) => {
        const color = arc.type === 'M4' ? '#00ff66' : '#0066ff';

        return (
          <line key={index}>
            <bufferGeometry>
              <bufferAttribute
                attach="attributes-position"
                args={[new Float32Array([arc.start.x, arc.start.y, arc.start.z, arc.end.x, arc.end.y, arc.end.z]), 3]}
              />
            </bufferGeometry>
            <shaderMaterial
              ref={(material) => {
                if (material) {
                  arcMaterialsRef.current[index] = material;
                }
              }}
              vertexShader={arcVertexShader}
              fragmentShader={arcFragmentShader}
              uniforms={{
                time: { value: 0 },
                color: { value: new THREE.Color(color) },
              }}
              transparent
              blending={THREE.AdditiveBlending}
            />
          </line>
        );
      })}

      <ComputeNode position={[1.5, 1.2, 0.5]} type="M4" intensity={2.5} />
      <ComputeNode position={[-1.2, -1.0, 1.0]} type="AMD" intensity={3} />
    </group>
  );
};

export default function HoloGlobe() {
  return (
    <div
      style={{
        width: '100vw',
        height: '100vh',
        backgroundColor: '#000',
        position: 'relative',
        overflow: 'hidden',
        fontFamily: 'monospace',
      }}
    >
      <Canvas camera={{ position: [0, 0, 6], fov: 45 }}>
        <ambientLight intensity={0.1} />
        <pointLight position={[10, 10, 10]} intensity={1} color="#00ffcc" />
        <Stars radius={100} depth={50} count={5000} factor={4} saturation={0} fade speed={1} />
        <EarthScene />
      </Canvas>

      <div
        style={{
          position: 'absolute',
          top: '40px',
          left: '40px',
          padding: '24px',
          borderLeft: '2px solid #22c55e',
          background: 'rgba(0, 0, 0, 0.4)',
          backdropFilter: 'blur(8px)',
          color: '#fff',
        }}
      >
        <h2 style={{ color: '#22c55e', fontSize: '12px', letterSpacing: '0.2em', margin: 0 }}>GLOBAL COMPUTE PRESSURE</h2>
        <div style={{ marginTop: '16px', display: 'flex', gap: '32px' }}>
          <div>
            <p style={{ color: '#6b7280', fontSize: '10px', margin: 0 }}>M4 CLUSTERS (GREEN)</p>
            <p style={{ color: '#fff', fontSize: '28px', fontWeight: 700, margin: 0 }}>
              42.8 <span style={{ fontSize: '12px' }}>Eflops</span>
            </p>
          </div>
          <div>
            <p style={{ color: '#6b7280', fontSize: '10px', margin: 0 }}>AMD HALO NODES (BLUE)</p>
            <p style={{ color: '#fff', fontSize: '28px', fontWeight: 700, margin: 0 }}>
              56.2 <span style={{ fontSize: '12px' }}>Eflops</span>
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
