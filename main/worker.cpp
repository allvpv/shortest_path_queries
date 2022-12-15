#include <iostream>
#include <boost/program_options.hpp>

#include "worker.hpp"

namespace po = boost::program_options;

po::variables_map get_options(int argc, char **argv) {
  po::options_description desc{"Syntax"};
  po::variables_map options;

  // clang-format off
  desc.add_options()
      ("port,p", po::value<u16>()->value_name("<u16>")->required(), "")
      ("help,h", "prints this message");
  // clang-format on

  po::store(po::parse_command_line(argc, argv, desc), options);

  if (options.count("help")) {
    std::cout << desc << "\n";
    return options;
  }

  po::notify(options);
  return options;
}

int main(int argc, char **argv) {
  try {
    po::variables_map options = get_options(argc, argv);

    if (options.count("help"))
      return EXIT_SUCCESS;

    ao::io_context ctx;

  } catch (po::error_with_option_name &e) {
    std::cerr << "Error: " << e.what() << ".\n";
    std::cerr << "Use `--help` to learn more.\n";
    return 1;

  } catch (std::runtime_error &e) {
    std::cerr << "Error: " << e.what() << ".\n";
    return 1;
  }
}
