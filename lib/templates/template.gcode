; Generated with `gcode-converter` for calibration
;
;      base_with: {{ calibration.base_width }}
;    base_height: {{ calibration.base_height }}
;  drawing_width: {{ calibration.drawing_width }}
; drawing_height: {{ calibration.drawing_height }}
;
{%- for point in channel %}
{%- if loop.first %}
g0 x{{ point.x }} y{{ point.y }}
{%- else%}
g0 x{{ point.x }} y{{ point.y }}
{%- endif %}
{%- endfor %}