import plotly
import plotly.express as px
from typing import List, Tuple

example_coordinates = [
    (43.7373476, 7.4240136),
    (43.7373825, 7.4241961),
    (43.7374014, 7.4243568),
    (43.7374193, 7.4245725),
    (43.7374283, 7.4246881),
    (43.7374405, 7.4248685),
    (43.7374518, 7.4249411),
    (43.7374699, 7.4250128),
    (43.7374832, 7.4250650),
    (43.7375256, 7.4251862),
]


def get_path_plot(
    coords: List[Tuple[float, float]]
) -> plotly.graph_objects._figure.Figure:
    lat, lon = zip(*coords)
    fig = px.line_mapbox(
        lat=lat,
        lon=lon,
        text=["start"] + [str(i) for i in range(1, len(lat) - 1)] + ["end"],
        center=dict(lat=43.73743, lon=7.424688),
        zoom=14,
        mapbox_style="open-street-map",
    )
    return fig
