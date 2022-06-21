output "instance_public_ip" {
  description = "Public IPv4 DNS of the EC2 instance"
  value       = aws_instance.squadbot.public_dns
}

output "ssh" {
  description = "ssh command to access EC2 instance"
  value       = format("ssh -i aws-key.pem ubuntu@%s", aws_instance.squadbot.public_dns)
}