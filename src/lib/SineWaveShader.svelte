<script>
    import { WebGlShader } from "svader";

const shaderCode = `#version 300 es
precision highp float;

out vec4 fragColor;

uniform vec2 u_resolution;
uniform vec2 u_offset;
uniform float time;


float cube_width = 20.0;
float wave_width = 2.5;
float PI = 3.1415926;
vec4 rainbow_color(int i)
{
    vec4 c;

    if (i == 0) c = vec4(0.0, 0.0, 0.0, 0.0); // transparent
    else if (i == 1) c = vec4(0, 64, 32, 255) / 255.0;     // dark green
    else if (i == 2) c = vec4(0, 128, 64, 255) / 255.0;    // mid green
    else if (i == 3) c = vec4(0, 200, 100, 255) / 255.0;   // bright green
    else if (i == 4) c = vec4(0, 220, 180, 255) / 255.0;   // teal
    else if (i == 5) c = vec4(0, 180, 255, 255) / 255.0;   // cyan
    else if (i == 6) c = vec4(0, 100, 255, 255) / 255.0;   // sky blue / indigo
    else c = vec4(0.0, 0.0, 0.0, 0.0);

    return c;
}

void main() {
    vec2 uv = gl_FragCoord.xy / u_resolution.xy;
    vec2 center = vec2(0.5, 0.5);
    float dist = distance(uv, center);
    uv = 1.0 - 2.0 * uv;

 
    //wave
    vec4 wave_color = vec4(0.);
    for(int i = 0;i<7;++i)
    {
    	float y = 0.1 * sin(uv.x*PI + float(i)/3. + .5*time*PI ) + uv.y+float(i)/10.0 - 0.5;
    	float wave = abs(1.0 / (y * u_resolution.y /wave_width));
    	wave_color += rainbow_color(i)*wave;
    }
    

    float vignette = smoothstep(0.45, 0.01, dist);  // Inner radius, outer radius
    wave_color *= vignette;
    //back ground
	fragColor = wave_color;

}
`;
</script>

<div class="shader-wrapper">
    <WebGlShader
        width="400px"
        height="100px"
    
        code={shaderCode}
        parameters={[
            { name: "u_resolution", value: "resolution" },
            { name: "u_offset", value: "offset" },
            { name: "time", value: "time"},
        ]}
    >
        <div class="fallback">WebGL not supported in this environment.</div>
    </WebGlShader>
</div>

<style>
    .shader-wrapper{
          position: absolute;
            top: 50%;
            left: 50%;
            transform: translate(-50%, -50%);
    }
</style>