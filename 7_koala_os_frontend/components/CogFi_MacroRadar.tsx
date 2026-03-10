import React, { useEffect, useRef, useState } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import { MeshDistortMaterial, Sphere, Stars } from '@react-three/drei';
import { AnimatePresence, motion } from 'framer-motion';
import * as THREE from 'three';

const TensorSphere = () => {
  const sphereRef = useRef<THREE.Mesh>(null);

  useFrame(({ clock }) => {
    if (sphereRef.current) {
      sphereRef.current.rotation.y = clock.getElapsedTime() * 0.05;
      sphereRef.current.rotation.z = clock.getElapsedTime() * 0.02;
    }
  });

  return (
    <Sphere ref={sphereRef} args={[1.5, 64, 64]} scale={1.2}>
      <MeshDistortMaterial
        color="#00ffcc"
        emissive="#004433"
        wireframe
        distort={0.3}
        speed={2}
        roughness={0.2}
      />
    </Sphere>
  );
};

export default function CogFi_MacroRadar() {
  const [gdp, setGdp] = useState(1.24);
  const [stakeRate, setStakeRate] = useState(58.4);
  const [tps, setTps] = useState(14200);
  const [slashAlert, setSlashAlert] = useState<string | null>(null);

  useEffect(() => {
    const dataInterval = setInterval(() => {
      setGdp((prev) => prev + Math.random() * 0.001);
      setStakeRate((prev) => Math.min(60, Math.max(50, prev + (Math.random() * 0.2 - 0.1))));
      setTps(Math.floor(10000 + Math.random() * 8000));
    }, 1000);

    const slashInterval = setInterval(() => {
      if (Math.random() > 0.8) {
        const fakeAddress = `0x${Math.random().toString(16).substring(2, 10).toUpperCase()}...`;
        setSlashAlert(
          `[EXECUTION] Persona Soulbound Slash triggered for Agent ${fakeAddress}. Cognitive score zeroed. Stake Burned.`,
        );
        setTimeout(() => setSlashAlert(null), 4000);
      }
    }, 6000);

    return () => {
      clearInterval(dataInterval);
      clearInterval(slashInterval);
    };
  }, []);

  return (
    <div
      style={{
        width: '100vw',
        height: '100vh',
        backgroundColor: '#050505',
        position: 'relative',
        overflow: 'hidden',
        fontFamily: 'monospace',
      }}
    >
      <Canvas camera={{ position: [0, 0, 4] }}>
        <ambientLight intensity={0.2} />
        <pointLight position={[10, 10, 10]} intensity={1.5} color="#00ffcc" />
        <Stars radius={100} depth={50} count={5000} factor={4} saturation={0} fade speed={1} />
        <TensorSphere />
      </Canvas>

      <div
        style={{
          position: 'absolute',
          top: 0,
          left: 0,
          width: '100%',
          height: '100%',
          pointerEvents: 'none',
          padding: '40px',
          boxSizing: 'border-box',
        }}
      >
        <div style={{ position: 'absolute', top: '40px', left: '40px', color: '#00ffcc' }}>
          <h2 style={{ fontSize: '14px', letterSpacing: '2px', opacity: 0.7 }}>GLOBAL AGENT GDP (EST.)</h2>
          <div style={{ fontSize: '42px', fontWeight: 'bold', textShadow: '0 0 10px rgba(0,255,204,0.5)' }}>
            ${gdp.toFixed(4)} <span style={{ fontSize: '20px' }}>Trillion</span>
          </div>
          <div style={{ fontSize: '12px', marginTop: '5px', color: '#888' }}>Tracking x402 &amp; AP2 Net Settlements</div>
        </div>

        <div style={{ position: 'absolute', top: '40px', right: '40px', color: '#ffb703', textAlign: 'right' }}>
          <h2 style={{ fontSize: '14px', letterSpacing: '2px', opacity: 0.7 }}>STAKE-TO-ACCESS LOCK-UP</h2>
          <div style={{ fontSize: '42px', fontWeight: 'bold', textShadow: '0 0 10px rgba(255,183,3,0.5)' }}>
            {stakeRate.toFixed(2)}%
          </div>
          <div style={{ fontSize: '12px', marginTop: '5px', color: '#888' }}>Supply Deflation Active</div>
        </div>

        <div style={{ position: 'absolute', bottom: '40px', left: '40px', color: '#fff' }}>
          <h2 style={{ fontSize: '14px', letterSpacing: '2px', opacity: 0.7, color: '#00b4d8' }}>EDGE NETTING (ZHA-ZHANG)</h2>
          <div style={{ display: 'flex', gap: '40px', marginTop: '10px' }}>
            <div>
              <div style={{ fontSize: '12px', color: '#888' }}>NETWORK TPS</div>
              <div style={{ fontSize: '24px', fontWeight: 'bold' }}>{tps.toLocaleString()}</div>
            </div>
            <div>
              <div style={{ fontSize: '12px', color: '#888' }}>COMPRESSION RATIO</div>
              <div style={{ fontSize: '24px', fontWeight: 'bold' }}>10,000 : 1</div>
            </div>
          </div>
        </div>

        <AnimatePresence>
          {slashAlert && (
            <motion.div
              initial={{ opacity: 0, scale: 0.8, y: 50 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 1.1, filter: 'blur(10px)' }}
              style={{
                position: 'absolute',
                top: '65%',
                left: '50%',
                transform: 'translate(-50%, -50%)',
                backgroundColor: 'rgba(255, 0, 51, 0.15)',
                border: '1px solid #ff0033',
                padding: '15px 30px',
                color: '#ff0033',
                fontSize: '16px',
                fontWeight: 'bold',
                boxShadow: '0 0 20px rgba(255,0,51,0.4)',
                backdropFilter: 'blur(4px)',
                textAlign: 'center',
                width: '600px',
              }}
            >
              ⚠️ {slashAlert}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}
