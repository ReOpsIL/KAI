import argparse

parser = argparse.ArgumentParser(description='Server description')
parser.add_argument('--port', type=int, help='Port number to run the server on')
args = parser.parse_args()

# Use args.port in the server logic where applicable