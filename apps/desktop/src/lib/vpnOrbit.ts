/** One immutable vertex buffer for the rotating map. Canvas2D also works when WebGL is unavailable. */
type GlobeDot = readonly [number, number, number, boolean, boolean];

export function packOrbitDots(dots: readonly GlobeDot[]) {
  return new Float32Array(dots.flatMap(([x, y, z, border, lime]) => [x, y, z, +border, +lime]));
}

export function createOrbitLayer(base: HTMLCanvasElement, vertices: Float32Array<ArrayBuffer>) {
  const layer = document.createElement('canvas');
  const gl = layer.getContext('webgl', {
    alpha: true,
    antialias: false,
    depth: false,
    stencil: false,
  });
  if (!gl) return;
  const program = gl.createProgram()!;
  function attach(type: number, source: string) {
    const shader = gl!.createShader(type)!;
    gl!.shaderSource(shader, source);
    gl!.compileShader(shader);
    gl!.attachShader(program, shader);
    gl!.deleteShader(shader);
  }
  attach(
    gl.VERTEX_SHADER,
    `
    attribute vec3 position;
    attribute vec2 style;
    uniform vec4 rotation;
    uniform vec4 viewport;
    uniform vec2 resolution;
    varying highp vec2 pixelCenter;
    uniform mediump float size;
    varying mediump vec3 color;
    void main() {
      float meridian = dot(position.xz,rotation.xy);
      float front = dot(vec2(position.y,meridian),rotation.zw);
      vec2 projected = vec2(dot(position.xz,vec2(rotation.y,-rotation.x)),
                            position.y*rotation.w-meridian*rotation.z);
      gl_Position = vec4(viewport.xy + projected*viewport.zw, front <= 0.0 ? 2.0 : 0.0, 1.0);
      pixelCenter = (gl_Position.xy + 1.0) * 0.5 * resolution;
      gl_PointSize = ceil(size) + 2.0;
      bool bright = style.x > 0.5 || front > 0.85;
      vec3 pink = bright ? vec3(255,0,157) : front < 0.46 ? vec3(135,48,94) : vec3(214,25,141);
      vec3 lime = bright ? vec3(180,255,0) : front < 0.46 ? vec3(88,109,38) : vec3(153,199,44);
      color = mix(pink,lime,style.y)/255.0;
    }`,
  );
  attach(
    gl.FRAGMENT_SHADER,
    `
    precision highp float;
    varying highp vec2 pixelCenter;
    uniform mediump float size;
    varying mediump vec3 color;
    void main() {
      vec2 delta = abs(gl_FragCoord.xy-pixelCenter) - vec2(size*0.25);
      float coverage = clamp(size*0.25-length(max(delta,0.0))+0.5,0.0,1.0);
      gl_FragColor = vec4(color*coverage,coverage);
    }`,
  );
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    gl.deleteProgram(program);
    gl.getExtension('WEBGL_lose_context')?.loseContext();
    return;
  }
  gl.useProgram(program);
  const buffer = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STATIC_DRAW);
  for (const [name, count, offset] of [
    ['position', 3, 0],
    ['style', 2, 12],
  ] as const) {
    const location = gl.getAttribLocation(program, name);
    gl.enableVertexAttribArray(location);
    gl.vertexAttribPointer(location, count, gl.FLOAT, false, 20, offset);
  }
  const rotation = gl.getUniformLocation(program, 'rotation');
  const viewport = gl.getUniformLocation(program, 'viewport');
  const size = gl.getUniformLocation(program, 'size');
  const resolution = gl.getUniformLocation(program, 'resolution');
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
  layer.setAttribute('aria-hidden', 'true');
  layer.className = 'orbit-layer';
  base.after(layer);
  return {
    hide() {
      layer.hidden = true;
    },
    draw(
      w: number,
      h: number,
      x: number,
      y: number,
      r: number,
      dot: number,
      lon: number,
      lat: number,
    ) {
      if (gl.isContextLost()) return false;
      if (layer.width !== base.width) layer.width = base.width;
      if (layer.height !== base.height) layer.height = base.height;
      if (layer.hidden) layer.hidden = false;
      gl.viewport(0, 0, layer.width, layer.height);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.uniform4f(rotation, Math.sin(lon), Math.cos(lon), Math.sin(lat), Math.cos(lat));
      gl.uniform4f(viewport, (2 * x) / w - 1, 1 - (2 * y) / h, (2 * r) / w, (2 * r) / h);
      gl.uniform2f(resolution, layer.width, layer.height);
      gl.uniform1f(size, (dot * layer.width) / w);
      gl.drawArrays(gl.POINTS, 0, vertices.length / 5);
      return true;
    },
    destroy() {
      gl.deleteBuffer(buffer);
      gl.deleteProgram(program);
      gl.getExtension('WEBGL_lose_context')?.loseContext();
      layer.remove();
    },
  };
}
