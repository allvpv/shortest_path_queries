# syntax=docker/dockerfile:1

FROM python:3.8-slim-buster

WORKDIR /partitioner

COPY requirements.txt requirements.txt
RUN pip3 install -r requirements.txt

COPY . .

CMD [ "python3", "main.py", "graphs/monaco.osm.gz", "--port", "50052", "--n_partitions=4" ]
