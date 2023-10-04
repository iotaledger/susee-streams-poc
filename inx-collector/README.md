# INX Collector Resources

This folder contains resources to run an
[inx-collector](https://github.com/teleconsys/inx-collector)
which is needed to use the susee-streams-poc applications with the
[Stardust update](https://wiki.iota.org/learn/protocols/stardust/introduction/) of the IOTA protocol
([IOTA mainnet](https://wiki.iota.org/get-started/introduction/iota/introduction/)
or [Shimmer Network](https://wiki.iota.org/get-started/introduction/shimmer/introduction/)).

The inx-collector maps Streams addresses to IOTA block-ids and additionally acts as a selective
permanode for all indexed blocks. The inx-collector consists of the following web services that
are run using docker virtualization and docker-compose:

* A [Hornet](https://wiki.iota.org/hornet/2.0-rc.6/welcome/) node
* The [INX Collector](https://github.com/teleconsys/inx-collector) plugin itself
* An [INX Proof of Inclusion](https://github.com/iotaledger/inx-poi) plugin

We cover two different usage scenarios, production and development.
This is described in the following sections in more detail.

## Use in production

As the inx-collector system includes a Hornet Node which communicates with other Nodes in the IOTA- or Shimmer-network
the system needs a publicly available domain name or static ip.

The minimum specs for the virtual or physical server are described on the Hornet
[Getting Started](https://wiki.iota.org/hornet/2.0-rc.6/getting_started/) page.

We recommend to use the **Ubuntu 22.04** operating system as the following installation
steps have been tested with this OS version.

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
Now we can start to install docker. A more detailed description of the
docker install and config steps can be found
[in this docker install howto](https://www.digitalocean.com/community/tutorials/how-to-install-and-use-docker-on-ubuntu-22-04)
.

```bash
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
  > sudo apt-get install docker-ce
  # Check the docker status after installation has been completed
  > sudo systemctl status docker
  # add the 'admin' user to the docker user group
  > sudo usermod -aG docker ${USER}
  > exit
```

The inx collector uses [minio](https://min.io) as object database. To access the
[minio console](https://min.io/docs/minio/linux/administration/minio-console.html)
via the host domain you need to configure a subdomain in the DNS system of your
[VPS](https://en.wikipedia.org/wiki/Virtual_private_server) (or physical server)
hoster. You need to add an 
[A record](https://en.wikipedia.org/wiki/List_of_DNS_record_types) or 
[CNAME record](https://en.wikipedia.org/wiki/CNAME_record)
for the subdomain "minio" that points to the ip-address resp. domainname of your host system.
Most VPS hoster provide a web-ui for DNS settings.
For example if your VPS can be accessed via the domain `example.com` the minio console
shall be accessible via `minio.example.com` after the installation steps have been finished.

Upload the content of the `inx-collector/hornet-install-resources` folder to your host system.
Before uploading it you may want to edit the values for ACME_EMAIL, NODE_HOST,
MINIO_ROOT_USER and MINIO_ROOT_PASSWORD values in the `env.hornet.example` file.
Alternatively you can edit those values after file upload off course. 

Please replace `<NODE_HOST>` with the domain name or static ip of your host system and enter the password for the
admin user when scp is executed. 
```bash
  # In the folder where this README.md is located (inx-collector folder)
  > scp hornet-install-resources/* admin@<NODE_HOST>:~
```

Please login again as admin user via ssh. We will now follow the steps described in the
[Install HORNET using Docker](https://wiki.iota.org/hornet/2.0-rc.6/how_tos/using_docker/)
howto.
```bash
  # in the admin home folder of your host system, check the folder content
  > ls -l
  # Make sure the following files exist:
  # * docker-compose.hornet.patch  
  # * docker-compose-https.patch
  # * env.hornet.example  
  # * prepare_docker.sh.patch  
  # * setup-hornet-node.sh
  
  # Execute the setup-hornet-node.sh script
  > ./setup-hornet-node.sh
  
  # If not done before edit the ACME_EMAIL, NODE_HOST, MINIO_ROOT_USER
  # and MINIO_ROOT_PASSWORD values in the env.hornet.example file using
  # an editor of your choice.
  # see https://wiki.iota.org/hornet/2.0-rc.6/how_tos/using_docker/#1-setup-environment
  # for more details
  > nano env.hornet.example
  
  # Copy the edited env.hornet.example into the hornet folder
  > cp env.hornet.example hornet/.env
  
  # Execute the prepare_docker.sh script in the hornet folder
  > cd hornet
  > sudo ./prepare_docker.sh
  
  # generate a password hash and salt for the hornet dashboard
  # see https://wiki.iota.org/hornet/2.0-rc.6/how_tos/using_docker/#5-set-dashboard-credentials  
  > docker compose run hornet tool pwd-hash
  
  # Enter a passwort and store it in your passwort safe (keepass or similar) for later use.
  # Choose a secure password because your server is part of a peer to peer network
  # and is seen by a lot of peers.
  #
  # As many VPS systems come with a preinstalled and running apache server
  # you may have problems because port 80 is already in use.
  # To disable and stop an already installed apache server:
  # > sudo systemctl disable apache2 && sudo systemctl stop apache2

  # Edit password hash and salt in the .env file using an editor of your choice.
  > nano .env
  
  # Replace the 0000000... values with the previously created hash and salt for
  # DASHBOARD_PASSWORD and DASHBOARD_SALT
  
  # We are now ready to start the services in the background
  > docker compose up -d
```

**After starting your node for the first time, please change the default 
grafana credentials User: admin Password: admin**

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