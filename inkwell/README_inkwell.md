# storyweave
Traverse the weave of stories

## The Idea

A local-first networked social bookmarking tool, with emergent social moderation and quality control at it's core. Curate your experience of the web as a series of stories, and follow your kin through their stories.

ephemeral webrtc connections for peers and topics

## The Implementation

Remember bookmarklets? I didn't.


Storyweave's core logic lives in a bookmarklet you can share around as you like! 


To generate the bookmarklet, we use a bootstrapper/fallback script that may be embedded on your choice of self-hosted static website to persist data for storyweave across origins, and allow users to configure the bookmarklet prior to use, if you set up the right elements on your page for it to inject into.


Users should be aware their bootstrapper (called an "inkwell") may intercept their storage requests and act as a fallback for signalling (or embed malicious code in the bookmarklet), but it is trivial to host a bootstrapper on localhost if you wish for that extra privacy and security. It is also possible to have inkwells with user auth for users to sync between devices - I will not implement this.


Users communicate with one another via webrtc - we currently assume initial signalling is done out-of-channel and then after they have bootstrapped a connection with another user, we use extant peers as a signalling network - we implement a very simple and naive sync protocol using webrtc datachannels.

##

