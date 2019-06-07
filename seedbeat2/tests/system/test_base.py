from seedbeat2 import BaseTest

import os


class Test(BaseTest):

    def test_base(self):
        """
        Basic test with exiting Seedbeat2 normally
        """
        self.render_config_template(
            path=os.path.abspath(self.working_dir) + "/log/*"
        )

        seedbeat2_proc = self.start_beat()
        self.wait_until(lambda: self.log_contains("seedbeat2 is running"))
        exit_code = seedbeat2_proc.kill_and_wait()
        assert exit_code == 0
