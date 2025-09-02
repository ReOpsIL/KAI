import argparse

parser = argparse.ArgumentParser(description='Server parameters')
parser.add_argument('--port', type=int, help='Port number to run the server on')
args = parser.parse_args()

# Use args.port in the server setup code