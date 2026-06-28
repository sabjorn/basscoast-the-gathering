// 3D MTG Card Viewer using Three.js

class CardViewer {
    constructor(containerId) {
        this.container = document.getElementById(containerId);
        if (!this.container) {
            console.error('Card viewer container not found');
            return;
        }

        this.scene = new THREE.Scene();
        this.camera = new THREE.PerspectiveCamera(
            45,
            this.container.clientWidth / this.container.clientHeight,
            0.1,
            1000
        );
        this.renderer = new THREE.WebGLRenderer({
            antialias: true,
            alpha: true
        });

        this.card = null;
        this.reflection = null;
        this.autoRotate = false;
        this.isDragging = false;
        this.previousMouseX = 0;

        this.currentArtistId = null;
        this.currentCardId = null;

        this.init();
    }

    init() {
        // Setup renderer
        this.renderer.setSize(this.container.clientWidth, this.container.clientHeight);
        this.renderer.setClearColor(0x000000, 0);
        this.container.appendChild(this.renderer.domElement);

        // Camera position
        this.camera.position.z = 6;
        this.camera.position.y = 0;

        // Lighting - balanced for card visibility
        this.ambientLight = new THREE.AmbientLight(0xffffff, 0.5);
        this.scene.add(this.ambientLight);

        this.directionalLight = new THREE.DirectionalLight(0xffffff, 0.5);
        this.directionalLight.position.set(2, 3, 5);
        this.scene.add(this.directionalLight);

        this.backLight = new THREE.DirectionalLight(0xffffff, 0.5);
        this.backLight.position.set(-2, -1, -3);
        this.scene.add(this.backLight);

        // Create card geometry (MTG card aspect ratio: 2.5 x 3.5 inches)
        const cardWidth = 2.5;
        const cardHeight = 3.5;
        const cornerRadius = 0.12; // MTG cards have rounded corners

        // Create rounded rectangle shape for the card
        const shape = new THREE.Shape();
        const x = -cardWidth / 2;
        const y = -cardHeight / 2;
        const width = cardWidth;
        const height = cardHeight;
        const radius = cornerRadius;

        shape.moveTo(x, y + radius);
        shape.lineTo(x, y + height - radius);
        shape.quadraticCurveTo(x, y + height, x + radius, y + height);
        shape.lineTo(x + width - radius, y + height);
        shape.quadraticCurveTo(x + width, y + height, x + width, y + height - radius);
        shape.lineTo(x + width, y + radius);
        shape.quadraticCurveTo(x + width, y, x + width - radius, y);
        shape.lineTo(x + radius, y);
        shape.quadraticCurveTo(x, y, x, y + radius);

        const geometry = new THREE.ShapeGeometry(shape);

        // Fix UV coordinates for proper texture mapping
        const uvAttribute = geometry.attributes.uv;
        const positions = geometry.attributes.position;

        for (let i = 0; i < uvAttribute.count; i++) {
            const px = positions.getX(i);
            const py = positions.getY(i);

            // Map from shape coordinates to UV coordinates (0,0) to (1,1)
            const u = (px - x) / width;
            const v = (py - y) / height;

            uvAttribute.setXY(i, u, v);
        }

        uvAttribute.needsUpdate = true;

        // Create a Group to hold front and back card faces
        this.card = new THREE.Group();

        // Load MTG card back texture immediately
        const textureLoader = new THREE.TextureLoader();
        // Use a placeholder MTG card back - we'll create our own static asset
        const cardBackUrl = '/static/images/mtg-card-back.jpg';

        // Create front material (will be updated with card image)
        this.frontMaterial = new THREE.MeshPhongMaterial({
            color: 0x333333,
            shininess: 10,
            side: THREE.DoubleSide
        });

        // Create back material with card back texture
        this.backMaterial = new THREE.MeshPhongMaterial({
            color: 0x333333, // Gray placeholder while loading
            shininess: 10,
            side: THREE.DoubleSide
        });

        // Load card back texture immediately and set it permanently
        textureLoader.load(
            cardBackUrl,
            (texture) => {
                // Set texture filtering for better quality
                texture.minFilter = THREE.LinearFilter;
                texture.magFilter = THREE.LinearFilter;

                this.backMaterial.map = texture;
                this.backMaterial.color.setHex(0xffffff);
                this.backMaterial.needsUpdate = true;
                console.log('Card back texture loaded and applied');
            },
            (progress) => {
                console.log('Loading card back:', Math.round((progress.loaded / progress.total) * 100) + '%');
            },
            (error) => {
                console.error('Failed to load card back:', error);
                console.error('URL:', cardBackUrl);
            }
        );

        // Create front face mesh
        this.cardFront = new THREE.Mesh(geometry, this.frontMaterial);
        this.cardFront.position.z = 0.01; // Front face

        // Create back face mesh (flipped)
        this.cardBack = new THREE.Mesh(geometry, this.backMaterial);
        this.cardBack.position.z = -0.01; // Back face
        this.cardBack.rotation.y = Math.PI; // Rotate 180 to face backward

        // Add both to the card group
        this.card.add(this.cardFront);
        this.card.add(this.cardBack);

        this.scene.add(this.card);

        // Create reflection plane below card
        const reflectionGeometry = new THREE.PlaneGeometry(cardWidth, 0.3);
        const reflectionMaterial = new THREE.MeshBasicMaterial({
            color: 0x000000,
            transparent: true,
            opacity: 0.15,
            side: THREE.DoubleSide
        });

        this.reflection = new THREE.Mesh(reflectionGeometry, reflectionMaterial);
        this.reflection.position.y = -(cardHeight / 2 + 0.2);
        this.reflection.rotation.x = Math.PI / 2;
        this.scene.add(this.reflection);

        // Mouse controls for rotation
        this.setupMouseControls();

        // Start animation loop
        this.animate();
    }

    setupMouseControls() {
        const canvas = this.renderer.domElement;

        canvas.addEventListener('mousedown', (e) => {
            this.isDragging = true;
            this.previousMouseX = e.clientX;
            canvas.style.cursor = 'grabbing';
        });

        canvas.addEventListener('mousemove', (e) => {
            if (this.isDragging) {
                const deltaX = e.clientX - this.previousMouseX;
                // Rotate on Y-axis (horizontal spin, matching auto-rotation)
                this.card.rotation.y += deltaX * 0.01;
                this.previousMouseX = e.clientX;
            }
        });

        canvas.addEventListener('mouseup', () => {
            this.isDragging = false;
            canvas.style.cursor = 'grab';
        });

        canvas.addEventListener('mouseleave', () => {
            this.isDragging = false;
            canvas.style.cursor = 'grab';
        });

        canvas.style.cursor = 'grab';
    }

    async loadCard(scryfallId, artistId) {
        if (!scryfallId) {
            console.error('No scryfall ID provided');
            return;
        }

        this.currentCardId = scryfallId;
        this.currentArtistId = artistId;

        try {
            // Fetch card data from our API
            const response = await fetch(`/mtg/cards/${scryfallId}`);
            if (!response.ok) {
                throw new Error(`Failed to fetch card: ${response.status}`);
            }

            const cardData = await response.json();

            // Determine image URL - use cached URI or construct from Scryfall ID
            let imageUrl = cardData.image_uri;
            if (!imageUrl) {
                // Fallback: construct Scryfall image URL from card ID
                // Format: https://cards.scryfall.io/normal/front/[first]/[second]/[full-uuid].jpg
                const id = scryfallId;
                imageUrl = `https://cards.scryfall.io/normal/front/${id[0]}/${id[1]}/${id}.jpg`;
            }

            // Load card image texture
            const textureLoader = new THREE.TextureLoader();
            textureLoader.load(
                imageUrl,
                (texture) => {
                    // Set texture filtering for better quality
                    texture.minFilter = THREE.LinearFilter;
                    texture.magFilter = THREE.LinearFilter;

                    // Update front material with texture
                    this.frontMaterial.map = texture;
                    this.frontMaterial.color.setHex(0xffffff); // Set to white to show texture properly
                    this.frontMaterial.needsUpdate = true;

                    // Reset rotation to show front face
                    this.card.rotation.set(0, 0, 0);

                    console.log('Card front loaded:', cardData.name);
                },
                undefined,
                (error) => {
                    console.error('Error loading card texture:', error);
                    console.error('Attempted URL:', imageUrl);
                }
            );
        } catch (error) {
            console.error('Error loading card:', error);
        }
    }

    animate() {
        requestAnimationFrame(() => this.animate());

        // Auto-rotation on Y-axis
        if (this.autoRotate && !this.isDragging) {
            this.card.rotation.y += 0.005;
        }

        this.renderer.render(this.scene, this.camera);
    }

    toggleRotation() {
        this.autoRotate = !this.autoRotate;
        const toggleButton = document.getElementById('toggle-rotation');
        if (toggleButton) {
            toggleButton.textContent = this.autoRotate ? 'Pause' : 'Play';
        }
        console.log('Auto-rotation:', this.autoRotate);
    }

    updateLighting(type, value) {
        switch(type) {
            case 'ambient':
                this.ambientLight.intensity = value;
                console.log('Ambient light:', value);
                break;
            case 'directional':
                this.directionalLight.intensity = value;
                console.log('Directional light:', value);
                break;
            case 'back':
                this.backLight.intensity = value;
                console.log('Back light:', value);
                break;
        }
    }

    handleWindowResize() {
        if (!this.container) return;

        const width = this.container.clientWidth;
        const height = this.container.clientHeight;

        this.camera.aspect = width / height;
        this.camera.updateProjectionMatrix();
        this.renderer.setSize(width, height);
    }
}

// Initialize viewer when DOM is loaded
let viewer = null;

document.addEventListener('DOMContentLoaded', () => {
    viewer = new CardViewer('card-viewer');

    // Make viewer globally accessible
    window.viewer = viewer;

    // Set up card selection change detection
    if (viewer && viewer.loadCard) {
        const originalLoadCard = viewer.loadCard.bind(viewer);
        viewer.loadCard = function(scryfallId, artistId) {
            originalLoadCard(scryfallId, artistId);

            // Wait for currentCardId to be set (texture load is async)
            // Poll with timeout instead of fixed delay
            let attempts = 0;
            const checkInterval = setInterval(() => {
                if (this.currentCardId === scryfallId || attempts++ > 20) {
                    clearInterval(checkInterval);

                    // Now trigger change detection
                    const saveButton = document.getElementById('save-changes');
                    if (saveButton && typeof window.checkForChanges === 'function') {
                        window.checkForChanges();
                    }
                }
            }, 50); // Check every 50ms, max 1 second
        };
    }

    // Setup toggle button
    const toggleButton = document.getElementById('toggle-rotation');
    if (toggleButton) {
        toggleButton.addEventListener('click', () => {
            if (viewer) {
                viewer.toggleRotation();
            }
        });
    }

    // Handle window resize
    window.addEventListener('resize', () => {
        if (viewer) {
            viewer.handleWindowResize();
        }
    });

    // Listen for HTMX events to wire up card clicks
    document.body.addEventListener('htmx:afterSwap', () => {
        // Wire up any new card-item elements that were just loaded
        const cardItems = document.querySelectorAll('.card-item');
        cardItems.forEach(item => {
            // Remove old listener if it exists to prevent duplicates
            item.replaceWith(item.cloneNode(true));
        });

        // Re-query and add fresh listeners
        document.querySelectorAll('.card-item').forEach(item => {
            item.addEventListener('click', () => {
                const scryfallId = item.dataset.scryfallId;
                const artistId = item.dataset.artistId;

                if (viewer && scryfallId) {
                    viewer.loadCard(scryfallId, artistId);

                    // Highlight selected card
                    document.querySelectorAll('.card-item').forEach(c => {
                        c.style.borderColor = '#444';
                    });
                    item.style.borderColor = '#00d4ff';

                    // Note: Selection is saved when user clicks "Save Changes" button
                }
            });
        });
    });
});
