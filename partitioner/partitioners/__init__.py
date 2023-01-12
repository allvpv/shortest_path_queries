def is_in_partition(lon, lat, partition):
    x_min, x_max = partition[0]
    y_min, y_max = partition[1]
    return x_min < lon <= x_max and y_min < lat <= y_max

