<script lang="ts">
  // The moving background: the walls of Majora's lair inside the Moon,
  // deep blue with colourful clouds drifting through. A small WebGL shader drawn at low
  // resolution (it's soft anyway), about 30 frames a second, paused when
  // the window is hidden. Still if the system asks for reduced motion.
  import { onMount } from "svelte";
  import { app } from "../lib/state.svelte";

  let canvas: HTMLCanvasElement;

  const VERTEX = `
    attribute vec2 p;
    void main() { gl_Position = vec4(p, 0.0, 1.0); }`;

  // Majora's lair: a deep blue wall with soft clouds drifting through it.
  // Two layers of fractal noise scroll in different directions; a third,
  // slower one picks the clouds' colour (yellow, orange, red, magenta,
  // green), and the whole thing slowly ebbs between nearly all blue and
  // vividly colourful.
  const FRAGMENT = `
    precision mediump float;
    uniform float t;
    uniform vec2 res;

    float hash(vec2 p) {
      return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
    }
    float noise(vec2 p) {
      vec2 i = floor(p), f = fract(p);
      vec2 u = f * f * (3.0 - 2.0 * f);
      return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x),
                 mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
    }
    float fbm(vec2 p) {
      float v = 0.0, a = 0.5;
      for (int i = 0; i < 4; i++) {
        v += a * noise(p);
        p = p * 2.03 + vec2(17.0, 9.0);
        a *= 0.5;
      }
      return v;
    }

    void main() {
      vec2 uv = gl_FragCoord.xy / res;
      vec2 p = (gl_FragCoord.xy - 0.5 * res) / res.y * 1.7;
      float time = t * 0.06;

      // Clouds: two drifting layers, warped by each other.
      vec2 drift1 = vec2(time * 0.9, time * 0.35);
      vec2 drift2 = vec2(-time * 0.6, time * 0.5);
      float a = fbm(p + drift1);
      float b = fbm(p * 1.4 + drift2 + a * 1.6);
      float cloud = smoothstep(0.4, 0.78, b);
      float shadow = smoothstep(0.5, 0.22, fbm(p * 0.8 - drift1 * 0.7 + 4.0));

      // Base: royal blue, darker toward the edges and in shadowy patches.
      vec3 blue = vec3(0.09, 0.09, 0.80);
      vec3 deep = vec3(0.02, 0.02, 0.28);
      vec2 c = uv - 0.5;
      float edge = smoothstep(0.25, 0.85, length(c * vec2(1.1, 1.3)));
      vec3 col = mix(blue, deep, max(edge * 0.85, shadow * 0.8));

      // Cloud colour sweeps smoothly through magenta, red, orange, yellow
      // and green (the colours of the MMR logo, which is based on this wall).
      // Varies a lot across the screen, so several colours show at once.
      float h = fbm(p * 1.1 + vec2(time * 0.3, -time * 0.2)) * 9.0 + p.x * 1.2 - p.y * 0.8 + time * 0.25;
      float hue = 0.08 + 0.25 * sin(h);
      vec3 cloudCol = clamp(abs(mod(hue * 6.0 + vec3(0.0, 4.0, 2.0), 6.0) - 3.0) - 1.0, 0.0, 1.0);
      cloudCol = mix(vec3(1.0), cloudCol, 0.85) * 0.95;

      // Colour only shows in patches that slowly fade in and out and drift,
      // so the wall stays mostly blue.
      float patch = smoothstep(0.36, 0.6, fbm(p * 0.8 + vec2(-time * 0.25, time * 0.15) + 11.0));
      float pulse = 0.65 + 0.35 * sin(time * 1.3 + fbm(p * 0.3) * 6.0);
      float amount = cloud * patch * pulse;
      col = mix(col, cloudCol, clamp(amount, 0.0, 0.9));
      // The clouds themselves lighten the blue a little even without colour.
      col += vec3(0.05, 0.06, 0.12) * cloud * (1.0 - patch);

      // Dark clouds drifting in front, a little faster than the rest.
      float darkCloud = smoothstep(0.42, 0.72, fbm(p * 1.1 + vec2(time * 1.3, -time * 0.45) + 30.0));
      col = mix(col, vec3(0.01, 0.01, 0.12), darkCloud * 0.65);

      gl_FragColor = vec4(col, 1.0);
    }`;

  onMount(() => {
    const gl = canvas.getContext("webgl", { antialias: false, premultipliedAlpha: false });
    if (!gl) return;
    const compile = (type: number, src: string) => {
      const sh = gl.createShader(type)!;
      gl.shaderSource(sh, src);
      gl.compileShader(sh);
      return sh;
    };
    const prog = gl.createProgram()!;
    gl.attachShader(prog, compile(gl.VERTEX_SHADER, VERTEX));
    gl.attachShader(prog, compile(gl.FRAGMENT_SHADER, FRAGMENT));
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) return;
    gl.useProgram(prog);
    const buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
    const loc = gl.getAttribLocation(prog, "p");
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
    const uT = gl.getUniformLocation(prog, "t");
    const uRes = gl.getUniformLocation(prog, "res");

    const SCALE = 0.25; // draw at a quarter resolution; CSS scales it up smoothly
    function resize() {
      const w = Math.max(1, Math.round(innerWidth * SCALE));
      const h = Math.max(1, Math.round(innerHeight * SCALE));
      if (canvas.width !== w || canvas.height !== h) {
        canvas.width = w;
        canvas.height = h;
        gl!.viewport(0, 0, w, h);
      }
    }

    const reduced = matchMedia("(prefers-reduced-motion: reduce)");
    const start = performance.now() - 60_000; // start mid-drift
    let frame = 0;
    let last = 0;
    function draw(now: number) {
      resize();
      gl!.uniform1f(uT, (now - start) / 1000);
      gl!.uniform2f(uRes, canvas.width, canvas.height);
      gl!.drawArrays(gl!.TRIANGLE_STRIP, 0, 4);
    }
    function loop(now: number) {
      frame = requestAnimationFrame(loop);
      if (document.hidden || now - last < 33) return;
      last = now;
      draw(now);
    }
    function restart() {
      cancelAnimationFrame(frame);
      if (reduced.matches || !app.motion) draw(performance.now());
      else frame = requestAnimationFrame(loop);
    }
    restart();
    const stop = $effect.root(() => {
      $effect(() => {
        void app.motion;
        restart();
      });
    });
    addEventListener("resize", restart);
    reduced.addEventListener("change", restart);
    return () => {
      cancelAnimationFrame(frame);
      stop();
      removeEventListener("resize", restart);
      reduced.removeEventListener("change", restart);
    };
  });
</script>

<canvas bind:this={canvas} class="lair" aria-hidden="true"></canvas>

<style>
  .lair {
    position: fixed;
    inset: 0;
    z-index: -1;
    width: 100vw;
    height: 100vh;
    pointer-events: none;
    opacity: var(--lair-opacity, 0.32);
    /* Fade toward the page colour so it glows rather than shouts. */
    mask-image: radial-gradient(130% 120% at 50% 40%, #000 30%, rgba(0, 0, 0, 0.55) 100%);
  }
  /* Light theme: the same colours as a faint pastel tint. */
  :global(:root[data-theme="light"]) .lair {
    --lair-opacity: 0.2;
  }
  @media (prefers-color-scheme: light) {
    :global(:root:not([data-theme="dark"])) .lair {
      --lair-opacity: 0.2;
    }
  }
</style>
