#!/usr/bin/env python3
import rclpy
from rclpy.node import Node
from sensor_msgs.msg import Imu
import random

class GreetingsPublisher(Node):
    def __init__(self):
        super().__init__("greetings_publisher")
        self.publisher_ = self.create_publisher(Imu, "/imu/imu", 10)
        self.timer_ = self.create_timer(1.0/5.0, self.publish_greetings)

    def publish_greetings(self):
        msg = Imu()
        msg.orientation.z = 10.0
        self.publisher_.publish(msg)

def main(args=None):
    rclpy.init(args=args)
    node = GreetingsPublisher()
    rclpy.spin(node)
    rclpy.shutdown()

if __name__ == "__main__":
    main()
