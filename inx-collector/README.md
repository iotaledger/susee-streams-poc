# INX Collector Resources

This folder contains resources to run an
[inx-collector](https://github.com/teleconsys/inx-collector)
which is needed to use the susee-streams-poc applications
[Stardust update](https://wiki.iota.org/shimmer/develop/explanations/what-is-stardust/) of the IOTA protocol
([IOTA mainnet](https://wiki.iota.org/shimmer/develop/explanations/what-is-shimmer/layer-1-landscape/)
or [Shimmer Network](https://wiki.iota.org/shimmer/develop/explanations/what-is-shimmer/introduction/)).

The inx-collector maps Streams address to IOTA block-ids and additionally acts as a selective
permanode for all indexed blocks. The inx-collector consists of the following web services that
are run using docker virtualization and docker-compose:

* A [Hornet](https://wiki.iota.org/shimmer/hornet/welcome/) node
* The [INX Collector](https://github.com/teleconsys/inx-collector) plugin itself
* An [INX Proof of Inclusion](https://github.com/iotaledger/inx-poi) plugin

We cover two different usage scenarios, production and development.
This is described in the following sections in more detail.

## Use in production

As the inx-collector system includes a Hornet Node which communicates with other Nodes in the IOTA- or Shimmer-network
the system needs a publicly available domain name or static ip.

The minimum specs for the virtual or physical server are described on the Hornet
[Getting Started](https://wiki.iota.org/shimmer/hornet/getting_started/) page.
The installation process described here is mainly based on the
[Install HORNET using Docker](https://wiki.iota.org/shimmer/hornet/how_tos/using_docker/) page.

Please take special care and follow the security recommendations and steps described in the
[Official Hornet Dockumentation](https://wiki.iota.org/shimmer/hornet/how_tos/using_docker/).


------------------------------------------
ATTENTION: The following content in this section is under construction

TODO: Rewrite text up from here

------------------------------------------

The script will process most steps described in the
[Hornet Dockumentation](https://wiki.iota.org/shimmer/hornet/how_tos/using_docker/)
with the following settings:
* Only HTPP is used
* The Traefik reverse proxy will use the default HTTP port 80
* Autopeering is used
* The created `hornet/data` folder will contain a subdirectory `inx-collector` for the inx-collector plugin.
* The Shimmer network is used
* No hornet dashboard credentials are created - please follow the steps
  [described here](https://wiki.iota.org/shimmer/hornet/how_tos/using_docker/#5-set-dashboard-credentials)
  to create these credentials
* No [additional monitoring](https://wiki.iota.org/shimmer/hornet/how_tos/using_docker/#6-enable-additional-monitoring)
  and no [wasp node](https://wiki.iota.org/shimmer/hornet/how_tos/using_docker/#7-enable-wasp-node) is enabled

For instructions on deploying the used
[object database minio](https://min.io)
to production environments,
see the [Deploy MinIO: Multi-Node Multi-Drive](https://min.io/docs/minio/linux/operations/install-deploy-manage/deploy-minio-multi-node-multi-drive.html#deploy-minio-distributed)
document page.

## Private tangle for development purposes

As a production system will be too expensive for development purposes we use a private tangle
with docker-compose instead. The system will be accessible on the development machine via localhost
or in the intranet.

The bash script `setup-private-tangle.sh` will process all needed steps to run a system as been
described on the [Run a Private Tangle](https://wiki.iota.org/shimmer/hornet/how_tos/private_tangle/)
page together with an [inx-collector](https://github.com/teleconsys/inx-collector) instance.

**The bash script shall only be used for test purposes**. Do not expose the ports
to the public internet.

The bash script will
[download a private tangle package](https://github.com/iotaledger/hornet/releases/download/v2.0.0-rc.6/HORNET-2.0.0-rc.6-private_tangle.tar.gz)
containing the latest docker-compose files that will be unpacked into a new subdirectory called `priv_tangle`.

After the `setup-private-tangle.sh` script has finished,
you can use the `priv_tangle` folder as root folder for the docker-compose-cli. Instead of using `docker compose up`
directly, please use the convenience script `run.sh` instead:

```bash
  # in the 'priv_tangle' subfolder:
  > ./run.sh
```

The connectivity of most of the provided services is documented in the
[privat tangle README file](./priv_tangle/README.md)
which is located in the created `priv_tangle` folder.

The REST api of the inx collector is available via localhost:9030.
For example, you can fetch a specific block using the api like this:
http://localhost:9030/block/block/block-id-goes-here

The [minio object database](https://min.io) can be accessed via http://127.0.0.1:9001
with username `your_access_id` and password `your_password`.