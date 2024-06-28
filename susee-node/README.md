# *SUSEE Node* Resources

## About

This folder contains resources to run single *SUSEE Node*
instance and redundant *SUSEE Node* setups.

A *SUSEE Node* provides all web services needed to 
run the *SUSEE Streams POC* and send *Sensor*
messages via a LoRaWAN communication infrastructure.

The *SUSEE Node* provides the following web services that
are run using docker virtualization and docker-compose.

*SUSEE Streams POC* applications:
* [*IOTA Bridge* REST API](../iota-bridge/README.md#iota-bridge-rest-api)
* [*Message Explorer* REST API](../management-console/README.md#run-message-explorer)
  implemented by the *Management Console* 
* [*AppServer Connector Mockup Tool*](../app-srv-connector-mock)

*IOTA Node* and *Selective Permanode* services:  
* [IOTA Hornet Node](https://wiki.iota.org/hornet/welcome/) node
* [INX Collector](https://github.com/teleconsys/inx-collector) plugin
* [INX Proof of Inclusion](https://github.com/iotaledger/inx-poi) plugin
* [Minio](https://min.io/) object database

Since the
[Stardust update](https://wiki.iota.org/learn/protocols/stardust/introduction/)
of the IOTA protocol in the
[IOTA mainnet](https://wiki.iota.org/get-started/introduction/iota/introduction/),
*IOTA Streams* can only be used with a self deployed *Tag Indexing Service*.
This *Tag Indexing Service* used for *SUSEE* is a modified version of the
[inx-collector](https://github.com/teleconsys/inx-collector)
by [Teleconsys](https://www.teleconsys.it/)
which also acts as a *Selective Permanode*.
The modified *INX Collector* stores *IOTA* blocks using
hashed *IOTA Streams* addresses (called *message index*)
as *Block Storage Keys*.

The source code of the modified *INX Collector* can be found here:
https://github.com/chrisgitiota/inx-collector/tree/streams-collector

Because data blocks being send via the *IOTA Tangle* without providing a
[Storage Deposit](https://wiki.iota.org/learn/protocols/stardust/core-concepts/storage-deposit/)
will be pruned after some time from *IOTA Nodes*,
data centric applications need a *Selective Permanode* functionality
which is also provided by the *INX Collector*.

A *Selective Permanode* filters application specific blocks
out of the *IOTA Tangle* and stores these blocks in a self
owned database. The *INX Collector* for *SUSEE* filters
*SUSEE* specific blocks by a configurable
[block tag](https://wiki.iota.org/tips/tips/TIP-0023/)
prefix and stores these blocks together with a *Proof of Inclusion*
in a [Minio](https://min.io/) object database.

Using the *Proof of Inclusion* that has been stored with the data block,
the authenticity and broadcasting time of the data payload contained in the
block can be proved any time in the future even when the
blocks have been pruned from the *IOTA Nodes*.
More details about *Proof of Inclusion* can be found in the
[main README](../README.md#proof-of-inclusion-or-why-is-iota-distributed-ledger-used).

Creating and validating a *Proof of Inclusion* is done using an
[INX Proof of Inclusion](https://github.com/iotaledger/inx-poi) plugin
which is also included in the *SUSEE Node*.

*INX Collector* and *INX Proof of Inclusion* are *INX Plugins*.
*INX Plugins* communicate with an *IOTA Node* via the
[INX Interface](https://github.com/iotaledger/inx)
which allows the plugins to have fast and extensive access to the
nodes internal *Tangle* data structures and *IOTA*
protocol communication.

To run *INX Plugins* the deployment of an own 
*IOTA Node* is obligatory. 
The *SUSEE Node* therefore also runs an
[IOTA Hornet Node](https://wiki.iota.org/hornet/welcome/)
and several additional *INX Plugins* that are needed for its use.

Here is an overview of the services contained in the
*SUSEE Node* and the communication between internal and 
external services: 

<br/>

<img src="SUSEE-Node-Services.png" alt="SUSEE-Node-Services" width="500"/>

<br/>

The following sections describe how to run single *SUSEE Node*
instances for production and develop purposes and how to
configure
[multiple *SUSEE Node* instances](#redundant-susee-node-setup)
to build a small and simple *SUSEE Node* 'cluster'.

## How to Deploy a SUSEE Node

We cover two different usage scenarios, production and development.
This is described in the sections 
[Use in production](#use-in-production)
and
[Private tangle for development purposes](private-tangle-for-development-purposes)
in more detail.

For the production scenario, the setup of the *SUSEE Node*
appliance and the *IOTA Node* + *Selective Permanode* services
is described below.
As services implemented by *SUSEE Streams POC* applications
are run using a *Docker Compose* setup described in the
[docker folder](../docker/README.md) of this repository,
the deploment of these services is described
[there](../docker/README.md#start-iota-bridge-and-message-explorer-as-public-available-service).

### Use in production

As the inx-collector system includes a Hornet Node which communicates with other Nodes in
the IOTA- or Shimmer-network,
the system needs a publicly available domain name.
All test systems have been run using an IPv4 address, so we recommend using an
IPv4 address, although it could be possible to use a IPv6 address.

The minimum specs for the virtual or physical server are:
* Virtual appliance (VPS) or physical server
* 4 virtual or phisical CPU Cores
* 16 GB RAM
* 50 GB SSD Diskspace
* accessible via a domain name

We recommend to use the **Ubuntu 22.04** operating system as the following installation
steps have been tested with this OS version.

#### Initial Server Setup

As your host system will be part of a permissionless peer to peer network its ip address
can be easily found. Therefore, please take special care on securing your host system
and follow best praxis security recommendations:
* https://www.digitalocean.com/community/tutorials/initial-server-setup-with-ubuntu-22-04
* https://blog.devolutions.net/2017/04/10-steps-to-secure-open-ssh/

After having created an admin user (named 'admin' in this readme) with sudo privilege (step 1 till 3 in 
[this initial server setup howto](https://www.digitalocean.com/community/tutorials/initial-server-setup-with-ubuntu-22-04)) 
please login as admin user via ssh.

Install ufw to configure the firewall as been described below. A more detailed description of the
ufw install and basic config steps can be found
[in this ufw firewall howto](https://www.digitalocean.com/community/tutorials/how-to-set-up-a-firewall-with-ufw-on-ubuntu-22-04)
.

```bash
  # in the admin home folder of your host system
  > sudo apt-get update
  > sudo apt-get install ufw
  > sudo ufw app list
  
  # Make sure that OpenSSH is listed in the 'Available applications' list
  
  > sudo ufw allow OpenSSH
  # ufw will ask you if you want to 'Proceed with operation'  - please press 'y' to proceed 
  > sudo ufw enable
  Command may disrupt existing ssh connections. Proceed with operation (y|n)? y
  Firewall is active and enabled on system startup
  # Check ufw status
  > sudo ufw status
  Status: active
  
  To                         Action      From
  --                         ------      ----
  OpenSSH                    ALLOW       Anywhere                  
  OpenSSH (v6)               ALLOW       Anywhere (v6)
```

#### Docker install

Now we can start to install docker. A more detailed description of the
docker install and config steps can be found
[in this docker install howto for Ubuntu](https://www.digitalocean.com/community/tutorials/how-to-install-and-use-docker-on-ubuntu-22-04).

**IMPORTANT**: If you are **not using Ubuntu, do not proceed with the docker install steps
described below**, but open the page linked above and use the OS switch to choose your OS.
Follow the instructions described there.

**IMPORTANT**: If your **host system is a VPS** please check the IPv4 network address
of your system. If the **network address is** in the range **"172.16.0.1/16"** or **"172.17.0.1/16"**
please follow the
[instructions given in this proxmox help thread](https://forum.proxmox.com/threads/docker-under-lxc-change-default-network.122634/)
and this
[serverfault discussion](https://serverfault.com/questions/916941/configuring-docker-to-not-use-the-172-17-0-0-range)
, to configure the **docker daemon to use an ip range of "172.30.0.1/16"**
for the docker bridge. Make sure to create the needed config file `/etc/docker/daemon.json`
before you execute `sudo apt-get install docker-ce`.
Otherwise, your  might be faced with a disabled network device
and your SSH connection will get lost. In this case you will need access to the 
virtualization hypervisor to access your system again after docker has been installed
or after docker compose up has been used,
to create the needed config file after the docker install.
A `docker_daemon_example.json` file is located in the `hornet-install-resources` folder. 

```bash
  # ---> ONLY FOR UBUNTU - See notes above <---
  
  # in the admin home folder of your host system
  > sudo apt update
  > sudo apt-get install apt-transport-https ca-certificates curl software-properties-common
  > curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
  
  > echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
  > sudo apt update
  # Check which docker-ce install package candidate is selected by apt for an docker-ce install.
  # Please have a look into the docker install howto linked above for more details.
  > apt-cache policy docker-ce
  
  # After having checked that the right candidate will be installed, we install docker here
  # IMPORTANT: ---> See above notes, if your host system is a VPS <--
  > sudo apt-get install docker-ce
  
  # Check the docker status after installation has been completed
  > sudo systemctl status docker
  # add the 'admin' user to the docker user group
  > sudo usermod -aG docker ${USER}
  > exit
```

#### Prepare subdomains for Minio and *INX Collector*

To run the *SUSEE Node* the following subdomains are needed to access specific services via https:

| Subdomain | Purpose |
|-----------|---------|
| minio | Minio API that can be used with the [Minio MC](https://min.io/docs/minio/linux/reference/minio-mc.html#create-an-alias-for-the-s3-compatible-service) or other Minio Clients. |
| minioui | [Minio Admin WebUI Console](https://min.io/docs/minio/linux/administration/minio-console.html) |
| collector | API of the *INX Collector* |

To configure the subdomains you need to add an 
[A record](https://en.wikipedia.org/wiki/List_of_DNS_record_types) or 
[CNAME record](https://en.wikipedia.org/wiki/CNAME_record)
for each subdomain that points to the ip-address resp. domainname of your host system.
Most VPS hoster provide a web-ui for DNS settings.
For example if your VPS can be accessed via the domain `example.com` the minio services
shall be accessible via the following subdomains after the installation steps have been
finished:

* `minioui.example.com`<br>
* `minio.example.com`<br>
* `collector.example.com`<br>

#### Install the Hornet docker environment 

Upload the content of the `inx-collector/hornet-install-resources` folder to your host system.
Please replace `<NODE_HOST>` with the domain name or static ip of your host system and enter the password for the
admin user when scp is executed.
```bash
  # In the folder where this README.md is located (inx-collector folder)
  > scp hornet-install-resources/* admin@<NODE_HOST>:~
```
Please login as admin user via ssh. The following steps are equivalent 
to the steps described in the
[Install HORNET using Docker](https://wiki.iota.org/hornet/how_tos/using_docker/)
howto.

**Important Note**: The setup-hornet-node.sh script that needs to be executed now will download all
needed resources to use the IOTA mainnet. If you want to use a different network
(for example Shimmernet) please edit the setup-hornet-node.sh file using an editor of
your choice and follow these steps:
* Search for the line `curl -L https://node-docker-setup.iota.org/iota | tar -zx`
* Replace the term `iota` in the path of the download url with one of the following
  network identifiers: `shimmer`,`testnet`, `iota-testnet`

After you have eventually edited the setup-hornet-node.sh script we are ready to run the
script:

```bash
  # in the admin home folder of your host system, check the folder content
  > ls -l
  # Make sure the following files exist:
  # * docker-compose-https.patch
  # * docker-compose-minio-client.yml
  # * docker-compose.hornet.patch
  # * docker_daemon_example.json   
  # * prepare_docker.sh.patch  
  # * setup-hornet-node.sh

  # If you want to use the Shimmer network instead of the IOTA Mainnet
  # Please edit the setup-hornet-node.sh as described above
  #  
  #    > nano setup-hornet-node.sh
  
  # Execute the setup-hornet-node.sh script
  > ./setup-hornet-node.sh
```

In the hornet folder created by `setup-hornet-node.sh`, create a password hash and salt for the hornet dashboard
as been described in the
[wiki](https://wiki.iota.org/hornet/how_tos/using_docker/#1-generate-dashboard-credentials).

Copy the output of the hornet pwd-hash tool into a temporary file or editor because
it will be needed during our next steps.

```bash
  > cd hornet

  # generate a password hash and salt for the hornet dashboard
  # see https://wiki.iota.org/hornet/how_tos/using_docker/#1-generate-dashboard-credentials  
  > docker compose run hornet tool pwd-hash
  
  # Enter a passwort and store it in your passwort safe (keepass or similar) for later use.
  # Choose a secure password because your server is part of a peer to peer network
  # and is seen by a lot of peers.
  #
  # As many VPS systems come with a preinstalled and running apache server
  # you may have problems because port 80 is already in use.
  # To disable and stop an already installed apache server:
  # > sudo systemctl disable apache2 && sudo systemctl stop apache2
```

Edit the following values in the
`hornet/env_template` file, that has been previously downloaded by the bash script.
You'll find more details about the edited variables in the `env_template` file.

```bash
  # Edit the env_template file using an editor of your choice
  # and implement the changes described below
  > nano env_template
```
If you are using https, uncomment and edit the variables of the https section.
If you are not using https, make sure the first line below is commented out.
* `COMPOSE_FILE`=docker-compose.yml:docker-compose-https.yml
* `ACME_EMAIL`
* `NODE_HOST`

Please also uncomment and edit the following variables:
* `HORNET_CONFIG_FILE=config.json` - Nothing to edit here, just uncomment
* `COMPOSE_PROFILES`- Search for the line defining the value `=${COMPOSE_PROFILES},monitoring` and uncomment it
   if you want to use grafana monitoring
* `DASHBOARD_USERNAME` - Use "susee-admin" for example
* `DASHBOARD_PASSWORD` - Enter the previously created hash value here
* `DASHBOARD_SALT` - Enter the previously created salt value here

You may also want to have a look into the
[setup-your-environment](https://wiki.iota.org/hornet/how_tos/using_docker/#2-setup-your-environment)
wiki page for Hornet to dive deeper into Hornet configuration.

Before storing the `env_template` file please append the following lines and
edit the values for `MINIO_ROOT_USER`, `MINIO_ROOT_PASSWORD` and `PEERCOLLECTOR_URL`
(more details regarding the `PEERCOLLECTOR_URL` can be found in the 
[Primary+Secondary *SUSEE-Node* Setup](#primarysecondary-susee-node-setup)
section below):

```dotenv
   ###################################
    # INX Collector and Minio section #
    ###################################
    
    # Edit the storage credentials for the minio object storage used by the inx-collector
    MINIO_ROOT_USER=minio-admin
    MINIO_ROOT_PASSWORD=minio-password-goes-here

    # Bucket name used to store streams messages by the inx-collector
    # Use a meaningfull name like one of these:
    #   * iota-mainnet
    #   * shimmernet-mainnet
    #   * acme-corp-private-testnet
    STORAGE_DEFAULT_BUCKET=iota-mainnet

    # Uncomment and edit the following line, if you are using a 'Primary+Secondary SUSEE-Node' setup.
    # The URL must include the "http" or "https" scheme.
    # Examples:
    #        PEERCOLLECTOR_URL=https://my-other-susse-node.org
    #        PEERCOLLECTOR_URL=http://127.0.0.1:9030
    #PEERCOLLECTOR_URL=https://my-other-susse-node.org
```

After having saved the `env_template` file in your editor
create an `.env` file from it.
```bash
  > cp env_template .env
```

Now we are able to prepare the data folder:
```bash
  # Execute the prepare_docker.sh script in the hornet folder
  > sudo ./prepare_docker.sh
```

Now we are able to start the `hornet` docker compose environment.
The first start of a hornet node can take a long time. Please have a look at the
[Hornet service stopps after 'docker compose up'](#hornet-service-stopps-after-docker-compose-up)
section while you are waiting for the `hornet` service to get healthy:
```bash   
  # In the hornet folder, start the services in the background
  > docker compose up -d
```

**After starting your node for the first time, please change the default 
grafana credentials.**<br>
Use the initial credentials User: `admin` Password: `admin` for the first login.

You should now be able to access the following endpoints:

* API: https://your-domain.com/api/routes
* HORNET Dashboard: https://your-domain.com/dashboard
* Grafana: https://your-domain.com/grafana
* Minio: https://minio.your-domain.com
* INX-Collector: http://your-domain.com:9030/block/block/block-id-goes-here

Please note: The REST API of the inx-collector is accessed via http and is not protected
by authentication so that anybody can use the API without restrictions.
This also applies to the REST API of the Hornet service.

Please note: For instructions on deploying the used
[object database minio](https://min.io)
to production environments as distributed system,
see the [Deploy MinIO: Multi-Node Multi-Drive](https://min.io/docs/minio/linux/operations/install-deploy-manage/deploy-minio-multi-node-multi-drive.html#deploy-minio-distributed)
documentation page.

#### Deploy *IOTA Bridge* and *Message Explorer*

Please follow the instructions described in the section
[Start IOTA Bridge and Message Explorer as public available service](../docker/README.md#start-iota-bridge-and-message-explorer-as-public-available-service)
of the [docker folder](../docker/README.md).

#### Update Docker Images

```bash
  # in the `hornet` or `susee-poc` folder of your SUSEE-Node
  > docker compose pull
  > docker compose up -d
```

### Private tangle for development purposes

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

The SUSEE-POC applications *IOTA Bridge* and *Management Console* need to know the
domain of the IOTA-Node which can be specified by the CLI `--node` argument. The domain
name is needed to build the URL of the IOTA-Node-API.
When the default value - '127.0.0.1' - for the `--node` argument is used, the
standard port of the IOTA-Node-API 14265 is automatically used to build the
URL of the IOTA-Node-API. This will work together with the private tangle for development
purposes.

If you are using the private tangle for development purposes and need to specify
the `--node` argument for a SUSEE-POC application explicitly, please use the
value `127.0.0.1`.

Also use the value `http://127.0.0.1:50000` for the `--iota-bridge` argument
of SUSEE-POC applications, in case it needs to be specified explicitly.
This should occur seldom as this is the default value.

The REST api of the inx collector is available via localhost:9030.
For example, you can fetch a specific block using the api like this:
http://localhost:9030/block/block/block-id-goes-here

The [minio object database](https://min.io) can be accessed via http://127.0.0.1:9001
with username `your_access_id` and password `your_password`.

To shut down the docker compose environment open a a second shell in the priv_tangle
folder and type:
```bash
  # in the 'priv_tangle' subfolder:
  > docker compose --profile "2-nodes" down
```

#### Profile using two minio instances

To test [Primary+Secondary *SUSEE-Node* Setup](#primarysecondary-susee-node-setup)
scenarios, two separated minio instances are needed. The docker-compose.yml
file provides a profile for this, called `2-minio`.

To start the private tangle using the `2-minio` profile:
```bash
  # in the 'priv_tangle' subfolder:
  > docker compose --profile "2-minio" up
```

The service 'minio-1' can be accessed as usual via http://127.0.0.1:9000 (API)
and http://127.0.0.1:9001 (Console).

The service 'minio-2' can be accessed via http://127.0.0.1:9002 (API)
and http://127.0.0.1:9003 (Console).

To stop the private tangle using the `2-minio` profile:
```bash
  # in the 'priv_tangle' subfolder:
  > docker compose --profile "2-minio" down
```

## Redundant SUSEE Node setup

Deployed to cost effective usual appliances (cloud hosted or onpremise,
VPS or physical), a *SUSEE Node* will often suffer from service outages
due to appliance downtimes of several minutes per week or due to
temporarily reduced appliance performance.

The service outages can be reduced by a simple redundancy architecture
using a primary and a secondary *SUSEE Node* deployed to two different
data centers. This will be called *Primary+Secondary Setup* in the following.

The available *SUSEE Node* can be run behind a load balancer, or the
*Application Server Connector* can do a simple
[failover](https://www.cloudflare.com/learning/performance/what-is-server-failover/).

The failover strategy used for the *Primary+Secondary Setup*
can be very simple and depends on the implementation of the
*Application Server Connector*.
The strategy must be chosen to take into account, which appliances,
networks and storage capabilities are available at each datacenter.

Following strategies can be considered. Mixing the strategies is also possible:
* *Prefer Primary Node*<br>
  In case the *IOTA Bridge* of the primary *SUSEE Node* 
  returns an error, the *Application Server Connector* will
  try to use the *IOTA Bridge* of the secondary *SUSEE Node*.
  The secondary *SUSEE Node* is only
  used in case of errors and only once (per error).
* *Spread Load*<br>
  The *Application Server Connector* distributes the work load
  using a simple round robin logic.
  In case an *IOTA Bridge* returns an error, the
  *Application Server Connector* tries to use the other
  *IOTA Bridge* instance. If both *IOTA Bridge* instances
  return errors, the *Application Server Connector* could
  retry to successfully transmit the request until a timeout
  is exceeded.
* *Use a dedicated Loadbalancer*<br>
  A [Loadbalancer](https://en.wikipedia.org/wiki/Load_balancing_(computing))
  would allow dynamic horizontal scaling but would introduce
  additional complexity and costs.

### Primary+Secondary *SUSEE-Node* Setup

The *SUSEE Node* is prepared to be used in a *Primary+Secondary Setup*.

<br/>

<img src="SUSEE-Node-Primary-Secondary-Setup.png" alt="Primary+Secondary Setup with two SUSEE-Nodes" width="800"/>

<br/>

#### Node Synchronisation

The *SUSEE Node* uses two technologies providing synchronisation mechanisms:

* *IOTA Network*<br>
  The *INX Collector* of each *SUSEE Node* will receive every data block
  that is send via the *IOTA Tangle*, regardless which *SUSEE Node* has been
  used to send the block. In theory, given the *IOTA Nodes*, *INX Collectors*
  and *Minio Databases* of all *SUSEE Nodes* would have no outages, no additional
  synchronization would be needed to have identical database contents.
* *Minio Database*<br>
  The *Minio Database* provides several synchronization features which can be
  mainly differentiated in:
  * [Server-Side Bucket Replication](https://min.io/docs/minio/linux/administration/bucket-replication.html)
  * [Client-Side Bucket Replication](https://min.io/docs/minio/linux/reference/minio-mc/mc-mirror.html#command-mc.mirror)

Applying two independent and not aligned synchronization mechanisms in parallel,
can result in access failures or performance issues, caused by unnecessary
processing due to the poorly aligned algorithms.
 
The *SUSEE Nodes* therefore use a synchronisation mechanism, tightly
aligned to the *IOTA Network* synchronization and the
([above decribed](#about)) *SUSEE Node* service
architecture.

We call this synchronisation mechanism '*Cluster wide block validation*'.

##### Cluster wide block validation

At the end of the block sending process of a *SUSEE* node,
every *Sensor* message, received by the *IOTA Bridge* and send via
the *IOTA Tangle*, must be stored as a block in the *Minio Database*.
For each `send-message` request, the *IOTA Bridge*
[validates](../iota-bridge/README.md#iota-bridge-error-handling-for-lorawan-node-endpoints)
the existence of the block, before the request is finished.

Validating the existence of the block in the *Minio Database*
is done via the *INX Collector*.

The *INX Collector* also provides
the option to configure a peer *INX Collector* instance in its
configuration using the `--peercollector.hostUrl` start parameter.
*Cluster wide block validation* is based on two simple principles:
* a block being validated in the local *Minio Database* will also
  be validated in the *Minio Database* of the *Peer INX Collector*
* a block that can't be found in the *Peer INX Collectors* *Minio*
  database, is fetched by the *Peer INX Collector* from the original
  *INX Collector*

The current implementation involves the primary and secondary
*SUSEE Node*. In general, these principles could be applied to an
arbitrary number of nodes.

To securely manage the *Block Storage Keys* that need to
be validated in the cluster, these keys are stored in the
*Minio Database* in the following *Minio Buckets*:
* `keys-to-send-to-peer-collector`<br>
  Used by the original *INX Collector* to store *Block Storage Keys*
  that could not be communicated to the *Peer INX Collector*
  due to a *Peer INX Collector* service outage.<br>
  The original *INX Collector* will try to communicate these
  keys later on and in case of success, remove the keys from the
  bucket.
* `objects-inspection-list`<br>
  Used by the *Peer INX Collector* to store *Block Storage Keys*
  that need to be validated in the local *Minio Database*.
  After the block-existence has been successfully validated, the
  *Block Storage Key* is removed. If the block can not be found
  in the local *Minio Database*, the block is fetched from the
  original *INX Collector*, stored in the local *Minio Database*
  and finally (on success) the *Block Storage Key* is removed
  from the bucket.
  
Please note that the association between original *INX Collector*
and *Peer INX Collector* is bidirectional.
The *INX Collector* of the secondary *SUSEE Node* is the *Peer INX Collector*
for the primary *SUSEE Node* and vice versa.

The *Cluster wide block validation* has been working reliably
and with less performance impact during all work load tests so far.

##### Minio Database synchronization

Although the synchronization features of the *Minio Database*
are not used to synchronize the primary and secondary *SUSEE Nodes*,
these features can be used for synchronization tasks of optional
additional nodes, for example for backup purposes.

For example the
[Minio client services for backup tasks](#minio-client-services-for-backup-tasks)
can be used to setup a backup appliance.

Alternatively you can choose from several AWS S3 compatible paid
cloud services.

## Node Maintenance

### Log files

The logs of the docker compose services can be accessed using the
[docker compose logs](https://docs.docker.com/reference/cli/docker/compose/logs/)
CLI command.

For fast log retrieval, the following expressions are often useful.
Replace `{SERVICENAME}` with the docker compose service name of
interest:

```bash
  # ------------------------------------------------------------------------------------
  # In the folder of the respective docker compose environment ('hornet' or 'susee-poc')
  # ------------------------------------------------------------------------------------

  # Show the last 500 entries of the services log
  > docker compose logs {SERVICENAME} --tail 500

  # Logs of the last 2 hours + follow
  > docker compose logs {SERVICENAME} --since 2h -f

  # Grep specific lines out of the logs
  > docker compose logs {SERVICENAME} --since 2h -f | grep "keywords to search for"

  # Logs between two specific timestamps
  docker compose logs {SERVICENAME} --since 2024-02-27T12:57:00Z --until 2024-02-27T13:57:00Z

  # Filter iota-bridge logs for errors and buffered messages (in the 'susee-poc' folder)
  docker compose logs iota-bridge --since 1h -f | grep 'error\|Adding\|send_buffered_message'
```

Docker compose service names of interest usually are:
* 'hornet' environment: `hornet`, `inx-collector`, `minio` 
* 'susee-poc' environment: `iota-bridge`, `mangement-console` 

##### Local File Logging Driver and log archival

In the [Docker install](#docker-install) section, the 
`docker_daemon_example.json` file, contained in the 
`hornet-install-resources` folder, has been recommended
as template for the active `docker_daemon.json` file.
Therefore, the following settings will cause the docker logs
on the *SUSEE Node* to be handled by the
[Local File Logging Driver](https://docs.docker.com/config/containers/logging/local/):

    "log-driver": "local",
    "log-opts": {
        "max-size": "50m",
        "max-file": "20"
    }

These `docker_daemon.json` settings result in logfiles, being
optimized for performance and disk use,
with a maximum size of 50 MB that will be
rotated until a maximum number of 20 files exist.

As been stated by the 
[Local File Logging Driver](https://docs.docker.com/config/containers/logging/local/)
webpage:

    The local logging driver uses file-based storage.
    These files are designed to be exclusively accessed
    by the Docker daemon.
    
    Interacting with these files with external tools may
    interfere with Docker's logging system and result in
    unexpected behavior, and should be avoided.

The logs should be archived before they get
lost due to the maximum number of 20 files. 
The safest and easiest way to archive the logs is to dump
them into a text file that is moved into an archive folder
later on:
```bash
  # ------------------------------------------------------------------------------------
  # In the folder of the respective docker compose environment ('hornet' or 'susee-poc')
  # ------------------------------------------------------------------------------------
  
  # Dump logs of May into a text file
  docker compose logs {SERVICENAME} --since 2024-05-01T00:00:00Z --until 2024-06-01T00:00:00Z > logs-{SERVICENAME}-2024-05.txt
```

To automate this procedure an appropriate *Logging Driver* (a
[list of Logging Driver](https://docs.docker.com/config/containers/logging/configure/#supported-logging-drivers)
is available on the docker website)
can be chosen, to be integrated in an eventually available
website monitoring tool.

#### How many binary log files exist?

To find out how many binary log files have been created
for a specific container, follow these steps:
* Find out the beginning of the container-id using `docker ps`
* List all container folders in `/var/lib/docker/containers`
  and find out the full container-id 
* List the content of the `local-logs` folder of the container

Here is an example session to list the binary log files for the `hornet` service:
```bash
    > docker ps

--> d90c2cb8c20d   iotaledger/hornet:2.0                          "/app/hornet -c conf…"   2 weeks ago   Up 8 days (healthy)             1883/tcp, 8081/tcp, 8091/tcp, 9029/tcp, 14265/tcp, 0.0.0.0:14626->14626/udp, :::14626->14626/udp, 0.0.0.0:15600->15600/tcp, :::15600->15600/tcp   hornet
    72b8787a62b5   minio/minio                                    "/usr/bin/docker-ent…"   2 weeks ago   Up 8 days                       0.0.0.0:9000-9001->9000-9001/tcp, :::9000-9001->9000-9001/tcp                                                                                     minio
    f89cda3a7a77   iotaledger/inx-dashboard:1.0                   "/app/inx-dashboard …"   2 weeks ago   Up 8 days                       9092/tcp                                                                                                                                          inx-dashboard
    353efc5ee9d3   traefik:v2.10                                  "/entrypoint.sh --pr…"   2 weeks ago   Up 8 days                       0.0.0.0:80->80/tcp, :::80->80/tcp, 0.0.0.0:443->443/tcp, :::443->443/tcp                                                                          traefik
    ...
    ...

    > sudo ls -l /var/lib/docker/containers

    drwx--x--- 5 root root 4096 Jun 20 10:08 ac80f1e440e0fe7de42cf0a5157cdf30e474894d5c8f61916f56d45ad5d63b39
--> drwx--x--- 5 root root 4096 Jun 28 14:44 d90c2cb8c20dee4fe6d6847bbc0a652f1d5a2e1a5862aed651b4b54362458104
    drwx--x--- 5 root root 4096 Jun 20 10:08 f89cda3a7a773daf798d373ea1c6324ed990f3ede57b17b635d19116cac367c4
    ...
    ...

    > sudo ls -l /var/lib/docker/containers/d90c2cb8c20dee4fe6d6847bbc0a652f1d5a2e1a5862aed651b4b54362458104/local-logs

    -rw-r----- 1 root root  846480 Jun 28 14:45 container.log
    -rw-r----- 1 root root 5181503 Jun 28 14:02 container.log.1.gz
    -rw-r----- 1 root root 5247221 Jun 26 20:15 container.log.2.gz
    -rw-r----- 1 root root 5221193 Jun 25 02:35 container.log.3.gz
    -rw-r----- 1 root root 5287911 Jun 23 09:00 container.log.4.gz
    -rw-r----- 1 root root 5351264 Jun 21 15:18 container.log.5.gz
```


### Manual Node Health Check

In the current version the health of a *SUSEE Node* can only
be checked manually.
To find out if a *SUSEE Node* is healthy, the following
checks and expressions can be helpful.
 
**Check the *IOTA Hornet* Dashboard**<br>
Dashboard URL:  https://{your-iotabridge.domain.com}/dashboard/<br>
Is the *IOTA Node* synced and healthy?

**Check the *IOTA Bridge* logs**<br>

```bash
    # In the 'susee-poc' folder of your SUSEE-Node

    # What happened in the last 30 minutes (append -f to follow log updates)
    > docker compose logs iota-bridge --since 30m
    # Show errors of the last day (remove -f if you don't want to follow log updates)
    > docker compose logs iota-bridge --since 24h -f | grep "error"
```

**Check the `docker stats`**<br>
The following expression sorts the `docker stats` table by the 4th column, which is 'MEM USAGE':
```bash
    > docker stats --no-stream --format "table {{.Name}}\t{{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}" | sort -k 4 -h
    
    NAME                             CONTAINER      CPU %     MEM USAGE / LIMIT
    inx-indexer                      ed4a7a9c48ef   0.00%     0B / 0B
    inx-poi                          5330988eee2c   0.00%     13.43MiB / 16GiB
    inx-mqtt                         b004b0d3c515   0.00%     13.52MiB / 16GiB
    inx-dashboard                    7371c2105ea2   0.33%     19.28MiB / 16GiB
    inx-spammer                      0489a2a751db   0.47%     19.33MiB / 16GiB
--> susee-poc-iota-bridge-1          3093e79089a8   0.00%     23.61MiB / 16GiB
    inx-participation                0eac489fc634   0.00%     29.17MiB / 16GiB
--> inx-collector                    4eb436248406   0.02%     45.52MiB / 16GiB
    traefik                          63e6ed8b06b9   0.03%     48.48MiB / 16GiB
    susee-poc-management-console-1   dba98e018b1e   0.00%     148.7MiB / 16GiB
--> minio                            8fa069fcef01   11.15%    3.069GiB / 16GiB
--> hornet                           9e9a9d6dcf0b   2.29%     3.82GiB / 16GiB
```

Check if the following containers are listed (means running) and if the CPU load and MEM USAGE makes sense:
* hornet
* minio
* susee-poc-iota-bridge-1
* inx-collector

**Check the available system memory with `free`**<br>
The alternative command `top` will show too less information, so better use `free`:
```bash
    > free
                   total        used        free      shared  buff/cache   available
    Mem:        16777216     1859952       76564         956    14840700    14916308
    Swap:              0           0           0
```

**Check the disk health with `df`:**<br>
```bash
    > df

    Filesystem        1K-blocks     Used Available Use% Mounted on
    /dev/ploop27901p1 495283928 34033880 440996820   8% /
    ....
    ....
```
Check if the available disk capacity is more than 80% resp. if the disk runs out of space.

**Check the disk size usage of your containers**<br>
The following expression will list the largest files and directories in
`/var/lib/docker/containers` sort by needed disk space.<br>
The size is shown in kilobytes (use "sudo du -am ...." for megabytes):
```bash
    > sudo du -a /var/lib/docker/containers | sort -n -r | head -n 10

    190144  /var/lib/docker/containers
    82452   /var/lib/docker/containers/9e9a9d6dcf0b0f239cf597af558feb6176e82eef7273b334c870725ba1a05afc
    82412   /var/lib/docker/containers/9e9a9d6dcf0b0f239cf597af558feb6176e82eef7273b334c870725ba1a05afc/local-logs
    35216   /var/lib/docker/containers/9e9a9d6dcf0b0f239cf597af558feb6176e82eef7273b334c870725ba1a05afc/local-logs/container.log
    34132   /var/lib/docker/containers/ed4a7a9c48ef2206461151af46e35f7d1701b2bab4fcf1003d41a0f279b793e1
    34096   /var/lib/docker/containers/ed4a7a9c48ef2206461151af46e35f7d1701b2bab4fcf1003d41a0f279b793e1/local-logs
    33304   /var/lib/docker/containers/4eb43624840639bcf7e3bbfb5f86cfc29b67c0f6bbc2dbed942ffa830b6e1d3f
    33264   /var/lib/docker/containers/4eb43624840639bcf7e3bbfb5f86cfc29b67c0f6bbc2dbed942ffa830b6e1d3f/local-logs
    33260   /var/lib/docker/containers/4eb43624840639bcf7e3bbfb5f86cfc29b67c0f6bbc2dbed942ffa830b6e1d3f/local-logs/container.log
    31224   /var/lib/docker/containers/0eac489fc63456ff9a294db6e76f4d71f34d97d1a2000cbb5841fe24b4cc9e11
```

**Check running docker containers with `docker ps`**<br>
```bash
    > docker ps

    CONTAINER ID   IMAGE                                      COMMAND                  CREATED       STATUS                          PORTS                                                                                                                                             NAMES
    8fa069fcef01   minio/minio                                "/usr/bin/docker-ent…"   2 weeks ago   Up 2 weeks                      0.0.0.0:9000-9001->9000-9001/tcp, :::9000-9001->9000-9001/tcp                                                                                     minio
    0489a2a751db   iotaledger/inx-spammer:1.0                 "/app/inx-spammer --…"   2 weeks ago   Up 2 weeks                      9092/tcp                                                                                                                                          inx-spammer
    5330988eee2c   iotaledger/inx-poi:1.0                     "/app/inx-poi --inx.…"   2 weeks ago   Up 2 weeks                                                                                                                                                                        inx-poi
    ....
    ....
```
Check if there are any services currently restarting ('STATUS' column). 

**About manually health checks**<br>
For a *Proof of Concept* (POC) System manually health checks are tolerable.
For a production system an 
[Automated Website Monitoring System](https://en.wikipedia.org/wiki/Website_monitoring)
providing a
[Docker Logging Driver](#local-file-logging-driver-and-log-archival),
log file persistence & analysis, permanent health checks and
alert infrastructure is mandatory.

### Node trouble shooting

If your *SUSEE Node* is not healthy, this will eventually be caused
by an unhealthy or not synced *IOTA Hornet Node* resp. the `hornet`
service of the 'hornet' docker compose environment.

All services in the 'hornet' and 'susee-poc' docker compose environments
are configured to restart automatically (unless they have been stopped),
so most exceptions of services will be healed by an automatic service restart.

As the *IOTA Hornet Node* manages a complex data structure, stored in
two large database files
(see [below](#hornet-service-stopps-due-to-corrupted-database))
and is continously synchronizing its state with other *IOTA Nodes*,
the node can end up in an unhealthy or unsynchronized state, that won't be fixed
by a service restart.

The following sections shall help with eventually problems. 

#### Hornet service stopps after 'docker compose up'

After having started the `hornet` docker compose environment using
`docker compose up -d`, the *IOTA Hornet Node* will need several minutes
to arrive in a healthy state.

The first start of a hornet node can take a long time. 
It's downloading and extracting a snapshot of around 1 GB,
quite intensive for CPU, RAM and disk.
Later on, if the `hornet` service is stopped & started
(with docker compose stop + start) or deleted & recreated
(with docker compose down + up), you usually won't need
to wait for long, given that the downtime of the service
has been in the range of one or two minutes.
The longer the downtime lasts, the more data need to be
synchronised from other peer *IOTA Nodes* until the node is
synchronised.
 
After having executed `docker compose up -d`, it's a good
idea to open a second shell in the 'hornet' folder and check
the hornet logs with `docker compose logs hornet -f`
to review the *Hornet* startup.

In general, if the `hornet` service is not listed as 'healthy'
several minutes after `docker compose up -d`,
a timeout could be exceeded, or the service
could have exited due to an unhandled exception.
Docker compose falsely reports failed timeouts as errors.

Reviewing the *Hornet* logs during the startup, will help
to find out the reason for eventually problems.

If you have not reviewed the *Hornet* logs during the startup:

* The easiest way to resolve an unhealthy `hornet` start
  due to timeouts, is to try `docker compose up hornet -d`
  again after approximately 5 Minutes.

* If the `hornet` service stopps a few seconds after
  `docker compose up -d` you should definetely have a look into
  the *Hornet* logs as described above.
  
The *Hornet* logs will hopefully contain hints, usually at the end
of the log, that help to find the reason for the issue.
Several typical reasons are discussed in the following sections.

#### Hornet service stopps due to a corrupted database

If the `hornet` service can't be started
due to a corrupted database, you need to 
stop all containers with `docker compose down`
and delete the corrupted database files:
```bash
    # In the 'hornet' folder of your SUSEE-Node
    > docker compose down
    ...
    > sudo rm -r data/database/tangle
    > sudo rm -r data/database/utxo
```

After having deleted the corrupted database files,
you also need to remove the outdated snapshot files
that have been downloaded while the hornet container
has been successfully started the last time
(otherwise *Hornet* would not sync later on because
it has used these outdated snapshot files instead of
downloading fresh ones):
```bash
    # In the 'hornet' folder of your SUSEE-Node
    > sudo rm data/snapshots/full_snapshot.bin
    > sudo rm data/snapshots/delta_snapshot.bin
```

#### Hornet service doesn't sync due to missing *INX Plugins*

If the hornet dashboard is available
(means you could open it in the browser) and the *Hornet Node* is not synced,
make sure that all configured plugins have been successfully started.

The following statement will list the running docker containers, sort by
their container name:
```bash
    > docker stats

    NAME                             CONTAINER      CPU %     MEM USAGE / LIMIT
    hornet                           9e9a9d6dcf0b   2.00%     4.503GiB / 16GiB
    inx-collector                    4eb436248406   0.01%     46.61MiB / 16GiB
    inx-dashboard                    7371c2105ea2   0.28%     21.15MiB / 16GiB
    inx-indexer                      ed4a7a9c48ef   0.00%     0B / 0B
    inx-mqtt                         b004b0d3c515   0.00%     14.6MiB / 16GiB
    inx-participation                0eac489fc634   0.00%     31.42MiB / 16GiB
    inx-poi                          5330988eee2c   0.00%     14.37MiB / 16GiB
    inx-spammer                      0489a2a751db   0.40%     18.99MiB / 16GiB
    minio                            8fa069fcef01   69.60%    4.831GiB / 16GiB
    susee-poc-iota-bridge-1          3093e79089a8   0.00%     26.41MiB / 16GiB
    susee-poc-management-console-1   dba98e018b1e   0.00%     212.4MiB / 16GiB
    traefik                          63e6ed8b06b9   0.03%     47.18MiB / 16GiB
```
Please check if all `inx-....` plugins listed above are listed in your console.
If there is an *INX Plugin* missing, check its service log using
`docker compose logs {SERVICENAME}`.

## Minio client services for backup tasks

A [MinIO Client](https://min.io/docs/minio/linux/reference/minio-mc.html)
can be used with docker to run
*MinIO Client* CLI commands
for one time data management tasks
or permanently running backup services.

### Minio Client commands for manual interaction

The docker compose file 
[`docker-compose-minio-client.yml`](hornet-install-resources/docker-compose-minio-client.yml)
in the `/susee-node/hornet-install-resources` folder
contains a service definition that can be used to run
[MinIO Client](https://min.io/docs/minio/linux/reference/minio-mc.html) 
commands.

This can be used for example to start a 
`minio-client` service in the `hornet` docker compose
environment using the
[`mirror`](https://min.io/docs/minio/linux/reference/minio-mc/mc-mirror.html)
*MinIO Client* CLI command.

Here is the documentation for the *MinIO Client service*, contained in the
`docker-compose-minio-client.yml` file:

    # =====================================================================
    #   IMPORTANT: Don't follow the instructions below using a production
    #              System. If you do it anyway, be sure you'll know what
    #              you are doing.
    # =====================================================================
    #
    # This file contains a service definition for a minio Client
    # service that can be copied into the docker-compose.yml file in
    # the hornet folder of your SUSEE-Node to run MINIO MC executions
    # in the docker compose environment.
    #
    # To enable the minio-client:
    # * Copy the minio-client service definition into the docker-compose.yml
    #   file in the hornet folder of your SUSEE-Node.
    #
    # * In the hornet folder of your SUSEE-Node:
    #   > docker compose up minio-client
    #
    #   Output will be:
    #      [+] Running 3/3
    #      Container traefik       Running                                                                                                                     0.0s
    #      Container minio         Running                                                                                                                     0.0s
    #      Container minio-client  Created                                                                                                                     0.2s
    #      Attaching to minio-client
    #      minio-client  | Added `localdb` successfully.
    #      minio-client  | [2024-05-12 15:44:14 UTC]     0B shimmer-mainnet-default/
    #      minio-client exited with code 0
    #
    # * Replace the statement '/usr/bin/mc ls localdb;'
    #   in the entrypoint definition of the minio-client service definition
    #   with a statement of your choice. Several examples can be found at the
    #   back of this comment block.
    #
    # * In the hornet folder of your SUSEE-Node:
    #   > docker compose start minio-client
    #
    # * After finishing work with the minio-client remove the service
    #   from the environment:
    #   > docker compose down minio-client
    #
    # * Delete the service definition from the docker-compose.yml file
    #   if you don't want it to be started on next 'docker compose up'
    #
    #
    # -----------------------------------------------------------------------
    # -------              Examples for mc executions             -----------
    # -----------------------------------------------------------------------
    #
    #   --------------------------------------
    # * Copy data from other minio/S3 sources.
    #   --------------------------------------
    #   Precondition: The environment variables MINIO_MIRROR_SOURCE_API_URL,
    #   MINIO_MIRROR_SOURCE_USER and MINIO_MIRROR_SOURCE_PASSWORD need to be
    #   defined in the .env file like this:
    #     ###########################
    #     # Minio Mirroring section #
    #     ###########################
    #     MINIO_MIRROR_SOURCE_USER=mirror-service-user
    #     MINIO_MIRROR_SOURCE_PASSWORD=SecretPasswordGoesHere
    #     MINIO_MIRROR_SOURCE_API_URL=https://minio.iotabridge.example.com
    #
    #   Execution statement for a one time mirroring:
    #      /usr/bin/mc alias set sourcedb ${MINIO_MIRROR_SOURCE_API_URL:-https://minio-api-url-must-be-defined-in-env.com} ${MINIO_MIRROR_SOURCE_USER:-susee-minio-source-admin} ${MINIO_MIRROR_SOURCE_PASSWORD:-susee-secret-source-password};
    #      /usr/bin/mc mirror sourcedb/${STORAGE_DEFAULT_BUCKET:-shimmer-mainnet-default} localdb/${STORAGE_DEFAULT_BUCKET:-shimmer-mainnet-default};
    #
    #   Alternative mirror statements:
    #   * One time mirroring with client side filtering (slow because of client side filtering):
    #       /usr/bin/mc mirror --newer-than 300s --older-than 25s --summary sourcedb/${STORAGE_DEFAULT_BUCKET:-shimmer-mainnet-default} localdb/${STORAGE_DEFAULT_BUCKET:-shimmer-mainnet-default};
    #   * Mirror + watch:
    #       /usr/bin/mc mirror --watch sourcedb/${MINIO_BACKUP_SOURCE_BUCKET:-iota-mainnet} localdb/${MINIO_BACKUP_TARGET_BUCKET:-shimmer-mainnet-default};
    #
    #   Minio mirror watch documentation:
    #   * https://min.io/docs/minio/linux/reference/minio-mc/mc-mirror.html#mc.mirror.-watch

### Independent docker compose environment for backup purposes

Having an independent *MinIO server service*
allows to use a *MINIO Client service* permanently,
for example to backup the data contained in the *Minio*
database of another *SUSEE Node*.

The
[`docker-compose-minio-backup.yml`](hornet-install-resources/docker-compose-minio-backup.yml)
file contained in the `/susee-node/hornet-install-resources` folder,
can be used
to create an independent docker compose environment
for backup purposes.

To set up such an environment:
* Create a new folder with a useful name describing the
  usecase of the environment ('minio-backup' for example).
* Copy the `docker-compose-minio-backup.yml` file into the new folder
  and rename it to `docker-compose.yml`
* In your new environment folder,
  create a `./data/minio` subfolder with access rights, so that
  the docker daemon has write-access to that folder
  (for example via 'sudo chown 65532:65532 ./data/minio')
* Create a .env file in the new folder, defining the
  following environment variables:
  * MINIO_ROOT_USER
  * MINIO_ROOT_PASSWORD
  * MINIO_BACKUP_SOURCE_API_URL
  * MINIO_BACKUP_SOURCE_USER
  * MINIO_BACKUP_SOURCE_PASSWORD
  * MINIO_BACKUP_SOURCE_BUCKET
  * MINIO_BACKUP_TARGET_BUCKET

The .env should look like this:

    MINIO_MIRROR_SOURCE_USER=mirror-service-user
    MINIO_MIRROR_SOURCE_PASSWORD=SecretPasswordGoesHere
    MINIO_MIRROR_SOURCE_API_URL=https://minio.iotabridge.example.com
    ...

The Minio User credentials can be configured in the minio
web UI console of the source minio server.
See the *Minio Console* documentation for
[User Access Keys](https://min.io/docs/minio/linux/administration/console/security-and-access.html#minio-console-user-access-keys)
for more details.

The new environment will run the following services:
* minio<br>
  The independent *MinIO server service*
* minio-client<br>
  The *Minio Client* running the `mirror --watch` command
* mc-restarter<br>
  Will restart the *Minio Client* every day

The `mirror --watch` *MinIO Client* CLI command sometimes fails
to synchronize all data. This happens when the data source
*SUSEE Node* runs under heavy workload or if the connection
is not stable.

To circumvent this problem the *MinIO Client service*
can be periodically restartet.

The `docker-compose-minio-backup.yml` file therefore contains a
service definition for a
*MINIO Client Restarter* service.

Here is the documentation for the *MINIO Client Restarter Service*, contained in the file:

    # Restarts the minio-client every 24 hours to force a mirroring
    # of the complete dataset.
    #
    # If the primary SUSEE-Node is under heavy workload the "mirror --watch"
    # synchronisation sometimes fails, so that several messages are missing in the
    # local minio database.
    #
    # Restarting the minio-client every 24 hours causes a new "mirror --watch"
    # process which starts with a complete dataset comparison.
    # This way the missing messages are synchronised.
