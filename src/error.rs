//! Errors generated by isolate.
error_chain!{
    // Wrappers for other error_chains.
    links {
    }

    // Wrappers for other errors.
    foreign_links {
    }

    // Internally defined errors.
    errors {
		// A stack allocation via mmap failed
		StackAllocation(err: ::errno::Errno) {
			description("Could not allocate stack")
			display("StackAllocation({})", err)
		}

		// A clone failed.
		Clone(err: ::errno::Errno) {
			description("Could not create thread clone")
			display("Clone({})", err)
		}

		// Failed to wait on a child.
		ChildWait(err: ::errno::Errno) {
			description("Error when waiting on a child")
			display("ChildWait({})", err)
		}
    }
}
