## &#x20;Introduction

Roccert (named after the Roc, a giant bird from Middle Eastern mythology, also known as the Peng in Chinese) is a Rust-based CLI tool for automatic SSL certificate renewal. It supports domain ownership validation via both HTTP-01 and DNS-01 challenges and can issue certificates for single domains, multiple domains, and wildcard domains. By checking the expiration time of existing certificates and integrating with the system's crontab scheduler, it enables fully automated certificate renewal without manual intervention.

## &#x20;Usage

Roccert solves HTTPS certificate management in the most intuitive way—10 minutes of setup for a permanent, hands-off solution.

```
roccert docs -l en  # Fetch the English documentation to learn the basic usage
roccert init        # Generate the config.toml configuration file required for the HTTP-01 challenge
roccert test        # Verify that the configuration is valid and test the process
roccert new         # Request a new certificate based on the configuration
roccert show -t     # View and verify the validity of the issued certificate

# Add a cron job for scheduled automatic renewal
5 21 * * * roccert renew /alice/roccert/config.toml >> /var/log/roccert.log 2>&1

```

## &#x20;Roadmap

As a domain certificate operations tool designed to eliminate manual intervention, we will continue focusing on three core principles: automation, lightweight design, and broad compatibility.

In the short term, we plan to expand support for additional Certificate Authority (CA) APIs, including ZeroSSL and Buypass Go SSL. We also aim to integrate APIs from more DNS providers, such as DnsPod, Xinnet, West.cn, ZDNS, Google Cloud DNS, Microsoft Azure DNS, Cloudflare, GoDaddy, and Namecheap.

Built on Rust’s high performance and memory safety, this project strives to deliver a lightweight, low-resource, and maintainable tool with a clean architecture. Through open-source collaboration, we hope to incorporate community ideas and contributions to gradually refine Roccert into a trusted certificate management assistant for developers and operators alike.

For questions, please contact: <hwpok@163.com> or <hwpok@qq.com>